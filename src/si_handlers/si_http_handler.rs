use futures::stream::StreamExt;
use http::HeaderValue;

use crate::swir_common;
use crate::utils::structs::*;
use hyper::{header, Body, HeaderMap, Method, Request, Response, StatusCode};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::str::FromStr;
use tokio::sync::{
    mpsc,
    oneshot
};

use hyper::header::HOST;

static X_CORRRELATION_ID_HEADER_NAME: &str = "x-correlation-id";

fn set_http_response(backend_status: BackendStatusCodes, response: &mut Response<Body>) {
    match backend_status {
        BackendStatusCodes::Ok(msg) => {
            *response.status_mut() = StatusCode::OK;
            *response.body_mut() = Body::from(msg);
        }
        BackendStatusCodes::Error(msg) => {
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            *response.body_mut() = Body::from(msg);
        }
        BackendStatusCodes::NoTopic(msg) => {
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            *response.body_mut() = Body::from(msg);
        }
        BackendStatusCodes::NoService(msg) => {
            *response.status_mut() = StatusCode::NOT_FOUND;
            *response.body_mut() = Body::from(msg);
        }
    }
}

fn extract_value_from_headers(header_name: String, headers: &HeaderMap<HeaderValue>) -> Option<String> {
    let header = header::HeaderName::from_lowercase(header_name.as_bytes()).unwrap();
    let maybe_header = headers.get(header);
    if let Some(value) = maybe_header {
        Some(String::from_utf8_lossy(value.as_bytes()).to_string())
    } else {
        None
    }
}

async fn get_whole_body(mut req: Request<Body>) -> Vec<u8> {
    let mut whole_body = Vec::new();
    while let Some(maybe_chunk) = req.body_mut().next().await {
        if let Ok(chunk) = &maybe_chunk {
            whole_body.extend_from_slice(chunk);
        }
    }
    whole_body
}

fn extract_correlation_id_from_headers(headers: &HeaderMap<HeaderValue>) -> Option<String> {
    extract_value_from_headers(String::from(X_CORRRELATION_ID_HEADER_NAME), headers)
}

fn extract_host_from_headers(headers: &HeaderMap<HeaderValue>) -> Option<String> {
    if let Some(host) = headers.get(HOST) {
        if let Ok(host_value) = host.to_str() {
            Some(host_value.to_string())
        } else {
            None
        }
    } else {
        None
    }
}

async fn service_invocation_processor(req: Request<Body>, from_client_to_si_sender: mpsc::Sender<RestToSIContext>) -> Response<Body> {
    let mut response = Response::new(Body::empty());
    let headers = req.headers().clone();
    debug!("service_invocation_processor : Headers {:?}", headers);

    let method = if let Ok(method) = swir_common::HttpMethod::from_str(&req.method().to_string()).map(|m| m as i32) {
        method
    } else {
        *response.status_mut() = StatusCode::BAD_REQUEST;
        return response;
    };

    let correlation_id = if let Some(correlation_id) = extract_correlation_id_from_headers(&headers) {
        correlation_id
    } else {
        *response.status_mut() = StatusCode::BAD_REQUEST;
        *response.body_mut() = Body::empty();
        return response;
    };

    let uri = req.uri().clone();

    debug!("service_invocation_processor : Correlation id {}", correlation_id);
    let whole_body = get_whole_body(req).await;

    let host = if let Some(host) = extract_host_from_headers(&headers) {
        host
    } else {
        *response.status_mut() = StatusCode::BAD_REQUEST;
        return response;
    };
    let service_name = host;

    let mut parsed_headers = HashMap::new();
    for (header, value) in headers.iter() {
        parsed_headers.insert(header.to_string(), String::from(value.to_str().unwrap()));
    }

    let req = swir_common::InvokeRequest {
        method,
        correlation_id: correlation_id.clone(),
        service_name: service_name.clone(),
        request_target: uri.path().to_string(),
        headers: parsed_headers,
        payload: whole_body,
    };

    let job = SIJobType::PublicInvokeHttp { req };
    let (local_sender, local_rx): (oneshot::Sender<SIResult>, oneshot::Receiver<SIResult>) = oneshot::channel();

    let ctx = RestToSIContext { job, sender: local_sender };
    let mut sender = from_client_to_si_sender;

    let res = sender.try_send(ctx);
    if let Err(e) = res {
        warn!("service_invocation_processor : Channel is dead {:?}", e);
        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        *response.body_mut() = Body::empty();
    } else {
        let response_from_service = local_rx.await;
        if let Ok(res) = response_from_service {
            debug!("service_invocation_processor : Got result {}", res);
            if let BackendStatusCodes::Ok(_) = res.status {
                if let Some(si_response) = res.response {
                    let result = si_response.result.unwrap();
                    if let Some(swir_common::InvokeStatus::Ok) = swir_common::InvokeStatus::from_i32(result.status) {
                        let mut headers = HeaderMap::new();
                        for (h, v) in si_response.headers.iter() {
                            let hn = header::HeaderName::from_bytes(h.as_bytes()).unwrap();
                            let hv = header::HeaderValue::from_str(v).unwrap();
                            headers.insert(hn, hv);
                        }

                        *response.headers_mut() = headers;
                        *response.body_mut() = Body::from(si_response.payload);
                        let status_code = u16::try_from(si_response.status_code).unwrap();
                        *response.status_mut() = StatusCode::from_u16(status_code).unwrap();
                    } else {
                        set_http_response(BackendStatusCodes::Error(result.msg), &mut response);
                    }
                } else {
                }
            } else {
                set_http_response(res.status, &mut response);
            }
        } else {
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            *response.body_mut() = Body::empty();
        }
    }
    info!("service_invocation_processor : {} {} end", correlation_id, service_name);
    response
}

pub async fn handler(req: Request<Body>, from_client_to_si_sender: mpsc::Sender<RestToSIContext>) -> Result<Response<Body>, hyper::Error> {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NOT_ACCEPTABLE;
    match *req.method() {
        Method::POST => {
            let response = service_invocation_processor(req, from_client_to_si_sender).await;
            Ok(response)
        }

        Method::PUT => {
            let response = service_invocation_processor(req, from_client_to_si_sender).await;
            Ok(response)
        }
        Method::DELETE => {
            let response = service_invocation_processor(req, from_client_to_si_sender).await;
            Ok(response)
        }

        Method::GET => {
            let response = service_invocation_processor(req, from_client_to_si_sender).await;
            Ok(response)
        }

        // The 404 Not Found route...
        _ => {
            warn!("Don't know what to do {} {}", &req.method(), &req.uri());
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}
