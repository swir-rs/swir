use crossbeam_channel::{Receiver, Sender, unbounded};
use futures_util::TryStreamExt;
use http::HeaderValue;
use hyper::{Body, Client, HeaderMap, Method, Request, Response, StatusCode};
use hyper::client::connect::dns::GaiResolver;
use hyper::client::HttpConnector;

use crate::utils::structs::{Job, MessagingResult, PublishRequest, RestToMessagingContext};
use crate::utils::structs::MessagingToRestContext;

fn validate_content_type(headers: &HeaderMap<HeaderValue>) -> Option<bool> {
    match headers.get(http::header::CONTENT_TYPE) {
        Some(header) => {
            if header == HeaderValue::from_static("application/json") {
                return Some(true);
            } else {
                return None;
            }
        }
        None => return None
    }
}

pub async fn handler(req: Request<Body>, sender: Sender<RestToMessagingContext>) -> Result<Response<Body>, hyper::Error> {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NOT_ACCEPTABLE;

    let headers = req.headers();
    debug!("Headers {:?}", headers);

    if validate_content_type(headers).is_none() {
        return Ok(response)
    }

    let (parts, body) = req.into_parts();

    debug!("Body {:?}", body);
    let url = parts.uri.clone().to_string();

    match (parts.method, parts.uri.path()) {
        (Method::POST, "/publish") => {
            let whole_body = body.try_concat().await;
            let response1 = whole_body.map(|payload| {
                    let p = &String::from_utf8_lossy(&payload);
                    PublishRequest { payload: p.to_string(), url: url }
                }).map(move |p| {
                debug!("{:?}", p);
                let (local_tx, local_rx): (Sender<MessagingResult>, Receiver<MessagingResult>) = unbounded();
                let job = RestToMessagingContext { job: Job::Publish(p), sender: local_tx.clone() };
                if let Err(e) = sender.send(job) {
                    warn!("Channel is dead {:?}", e);
                    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    *response.body_mut() = Body::empty();
                }

                let r = local_rx.recv();
                debug!("Got result {:?}", r);
                if let Ok(res) = r {
                    *response.body_mut() = Body::from(res.result);
                    *response.status_mut() = StatusCode::OK;
                } else {
                    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    *response.body_mut() = Body::empty();
                }
                response
            });
            return response1
        }

        (Method::POST, "/subscribe") => {
            let whole_body = body.try_concat().await;
            let response2 = whole_body.map(|payload| {
                    let p = &String::from_utf8_lossy(&payload);
                    info!("Payload is {:?}", &p);
                    serde_json::from_str(&p)
                }).map(move |p| {
                match p {
                    Ok(json) => {
                        info!("{:?}", json);
                        let (local_tx, local_rx): (Sender<MessagingResult>, Receiver<MessagingResult>) = unbounded();
                        let job = RestToMessagingContext { job: Job::Subscribe(json), sender: local_tx.clone() };
                        info!("About to send to kafka processor");
                        if let Err(e) = sender.send(job) {
                            warn!("Channel is dead {:?}", e);
                            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                            *response.body_mut() = Body::empty();
                        }
                        info!("Waiting for response from kafka");
                        let r = local_rx.recv();
                        info!("Got result {:?}", r);
                        if let Ok(res) = r {
                            *response.body_mut() = Body::from(res.result);
                            *response.status_mut() = StatusCode::OK;
                        } else {
                            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                            *response.body_mut() = Body::empty();
                        }
                    }
                    Err(e) => {
                        warn!("{:?}", e);
                        *response.status_mut() = StatusCode::BAD_REQUEST;
                        *response.body_mut() = Body::from(e.to_string());
                    }
                }
                response
            });
            return response2
        }

        // The 404 Not Found route...
        _ => {
            let mut not_found = Response::default();
            *response.status_mut() = StatusCode::NOT_FOUND;
            return Ok(not_found);
        }
    };

}


fn send_request(client: Client<HttpConnector<GaiResolver>>, payload: MessagingToRestContext) {
    let uri = payload.uri;
    let url = uri.parse::<hyper::Uri>().unwrap();

    let p = payload.payload.clone();
    let req = Request::builder()
        .method("POST")
        .uri(url)
        .header(hyper::header::CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .body(Body::from(payload.payload))
        .expect("request builder");

//    let sender = payload.sender.clone();
//    let err_sender = payload.sender.clone();

    let f = async move {
        let p = p.clone();
        info!("Making request for {}", String::from_utf8_lossy(&p));
        let resp = client.request(req).await;
        let res = resp.map(move |res| {
                debug!("Status POST to the client: {}", res.status());
//                let mut status = "All good".to_string();
                if res.status() != hyper::StatusCode::OK {
                    warn!("Error from the client {}", res.status());
//                    status = "Invalid response from the client".to_string();
                }
//                if let Err(e) = sender.send(MessagingResult { status: u32::from(res.status().as_u16()), result: status }) {
//                    warn!("Problem with an internal communication {:?}", e);
//                }
//                res.into_body().concat2()
            res.into_body()
        }).map(|_| {})
                .map_err(move |err| {
                    eprintln!("Error {}", err);
//                    if let Err(e) = err_sender.send(MessagingResult { status: 1, result: "Something is wrong".to_string() }) {
//                        warn!("Problem with an internal communication {:?}", e);
//                    }
                });
    };
    hyper::rt::spawn(f);
}

pub async fn client_handler(rx: Receiver<MessagingToRestContext>) {
    let client = hyper::Client::builder().keep_alive(true).build_http();
    for payload in rx.iter() {
        send_request(client.clone(), payload);
    }
}