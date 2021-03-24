use crate::swir_common;
use crate::utils::{structs::*, tracing_utils,metric_utils,metric_utils::bump_http_response_counters};
use futures::StreamExt;
use std::time::SystemTime;
use http::HeaderValue;
use hyper::{
    client::{connect::dns::GaiResolver, HttpConnector},
    header, Body, Client, HeaderMap, Method, Request, Response, StatusCode,
};
use serde::Deserialize;
use std::{collections::HashMap, str::FromStr, sync::Arc};
use tracing::Span;
use tracing_futures::Instrument;

use tokio::sync::{mpsc, oneshot, Mutex};
use opentelemetry::KeyValue;

#[derive(Debug)]
enum PersistenceOperationType {
    Store,
    Retrieve,
    Delete,
}

#[derive(Debug, Deserialize)]
pub struct ServiceInvokeRequestHttp {
    pub method: String,
    pub request_target: String,
    pub headers: std::collections::HashMap<String, String>,
    pub payload: String,
}

static X_CORRRELATION_ID_HEADER_NAME: &str = "x-correlation-id";
static X_DATABASE_NAME_HEADER_NAME: &str = "x-database-name";
static X_DATABASE_KEY_HEADER_NAME: &str = "x-database-key";
static X_TOPIC_HEADER_NAME: &str = "topic";

fn extract_value_from_headers(header_name: String, headers: &HeaderMap<HeaderValue>) -> Option<String> {
    let header = header::HeaderName::from_lowercase(header_name.as_bytes()).unwrap();
    let maybe_header = headers.get(header);
    if let Some(value) = maybe_header {
        Some(String::from_utf8_lossy(value.as_bytes()).to_string())
    } else {
        None
    }
}

fn extract_topic_from_headers(headers: &HeaderMap<HeaderValue>) -> Option<String> {
    extract_value_from_headers(String::from(X_TOPIC_HEADER_NAME), headers)
}

fn extract_correlation_id_from_headers(headers: &HeaderMap<HeaderValue>) -> Option<String> {
    extract_value_from_headers(String::from(X_CORRRELATION_ID_HEADER_NAME), headers)
}

fn extract_database_name_from_headers(headers: &HeaderMap<HeaderValue>) -> Option<String> {
    extract_value_from_headers(String::from(X_DATABASE_NAME_HEADER_NAME), headers)
}

fn extract_database_key_from_headers(headers: &HeaderMap<HeaderValue>) -> Option<String> {
    extract_value_from_headers(String::from(X_DATABASE_KEY_HEADER_NAME), headers)
}

fn find_channel_by_topic<'a>(
    client_topic: &'a str,
    from_client_to_backend_channel_sender: &'a HashMap<String, mpsc::Sender<RestToMessagingContext>>,
) -> Option<&'a mpsc::Sender<RestToMessagingContext>> {
    from_client_to_backend_channel_sender.get(client_topic)
}

fn find_channel_by_database_name<'a>(
    database_name: &'a str,
    from_client_to_persistence_sender: &'a HashMap<String, mpsc::Sender<RestToPersistenceContext>>,
) -> Option<&'a mpsc::Sender<RestToPersistenceContext>> {
    from_client_to_persistence_sender.get(database_name)
}

fn validate_content_type(headers: &HeaderMap<HeaderValue>) -> Option<bool> {
    match headers.get(http::header::CONTENT_TYPE) {
        Some(header) => {
            if header == HeaderValue::from_static("application/json") {
                debug! {"Found header {:?}",header}
                Some(true)
            } else {
                None
            }
        }
        None => None,
    }
}

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

async fn get_whole_body(mut req: Request<Body>) -> Vec<u8> {
    let mut whole_body = Vec::new();
    while let Some(maybe_chunk) = req.body_mut().next().await {
        if let Ok(chunk) = &maybe_chunk {
            whole_body.extend_from_slice(chunk);
        }
    }
    whole_body
}

async fn sub_unsubscribe_handler(
    is_subscribe: bool,
    whole_body: Vec<u8>,
    correlation_id: String,
    from_client_to_backend_channel_sender: &HashMap<String, mpsc::Sender<RestToMessagingContext>>,
    to_client_sender: mpsc::Sender<BackendToRestContext>,
) -> Response<Body> {
    let wb = String::from_utf8_lossy(&whole_body);
    if is_subscribe {
        info!("Subscribe {} ", wb);
    } else {
        info!("Unsubscribe {} ", wb);
    }
    let maybe_json = serde_json::from_slice(&whole_body);
    match maybe_json {
        Ok(json) => sub_unsubscribe_processor(is_subscribe, json, correlation_id, &from_client_to_backend_channel_sender, to_client_sender).await,
        Err(e) => {
            warn!("Unable to parse body {:?}", e);
            let mut response = Response::new(Body::from(e.to_string()));
            *response.status_mut() = StatusCode::BAD_REQUEST;
            response
        }
    }
}

async fn service_invocation_processor(correlation_id: String, path: String, req: Request<Body>, from_client_to_si_sender: mpsc::Sender<RestToSIContext>) -> Response<Body> {
    let mut response = Response::new(Body::empty());

    let service_name = path["/serviceinvocation/invoke/".len()..].to_string();
    info!("service_invocation_processor: {} {:?}", correlation_id, service_name);

    let whole_body = get_whole_body(req).await;
    let wb = String::from_utf8_lossy(&whole_body);
    debug!("Body {}", wb);
    let maybe_json = serde_json::from_slice(&whole_body);

    let response = if let Ok(json) = maybe_json {
        let client_req: ServiceInvokeRequestHttp = json;

        let req = if let Ok(bytes) = base64::decode(&client_req.payload) {
            let method = if let Ok(method) = swir_common::HttpMethod::from_str(&client_req.method) {
                method
            } else {
                *response.status_mut() = StatusCode::BAD_REQUEST;
                return response;
            };
            swir_common::InvokeRequest {
                method: method as i32,
                correlation_id: correlation_id.clone(),
                service_name: service_name.clone(),
                request_target: client_req.request_target.to_owned(),
                headers: client_req.headers.to_owned(),
                payload: bytes,
            }
        } else {
            *response.status_mut() = StatusCode::BAD_REQUEST;
            return response;
        };

        let job = SIJobType::PublicInvokeHttp { req };
        let (local_sender, local_rx): (oneshot::Sender<SIResult>, oneshot::Receiver<SIResult>) = oneshot::channel();

        let ctx = RestToSIContext {
            job,
            sender: local_sender,
            span: Span::current(),
        };
        let sender = from_client_to_si_sender;

        let res = sender.try_send(ctx);
        if let Err(e) = res {
            warn!("Channel is dead {:?}", e);
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            *response.body_mut() = Body::empty();
        } else {
            debug!("Waiting for response");
            let response_from_service = local_rx.await;
            debug!("Got result {:?}", response_from_service);
            if let Ok(res) = response_from_service {
                set_http_response(res.status, &mut response);
                if let Some(si_response) = res.response {
                    *response.body_mut() = Body::from(format!("{}", si_response));
                };
            } else {
                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                *response.body_mut() = Body::empty();
            }
        }
        response
    } else {
        debug!("Invalid Json {:?}", maybe_json);
        *response.status_mut() = StatusCode::NOT_ACCEPTABLE;
        response
    };
    info!("service_invocation_processor: {} {:?} end", correlation_id, service_name);
    response
}

async fn persistence_processor(
    op_type: PersistenceOperationType,
    correlation_id: String,
    headers: &HeaderMap<HeaderValue>,
    req: Request<Body>,
    from_client_to_persistence_sender: &HashMap<String, mpsc::Sender<RestToPersistenceContext>>,
) -> Response<Body> {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NOT_ACCEPTABLE;
    let database_name = extract_database_name_from_headers(headers).unwrap();
    let key = extract_database_key_from_headers(headers).unwrap();
    let whole_body = get_whole_body(req).await;
    let msg = format!("op {:?} -> {} {}", op_type, database_name, key);

    let maybe_channel = find_channel_by_database_name(&database_name, from_client_to_persistence_sender);
    let sender = if let Some(channel) = maybe_channel {
        channel.clone()
    } else {
        set_http_response(BackendStatusCodes::NoTopic("No mapping for this topic".to_string()), &mut response);
        return response;
    };

    let (local_tx, local_rx): (oneshot::Sender<PersistenceResult>, oneshot::Receiver<PersistenceResult>) = oneshot::channel();

    let res = match op_type {
        PersistenceOperationType::Store => {
            let sr = StoreRequest {
                correlation_id,
                payload: whole_body,
                table_name: database_name,
                key: key,
            };

            let job = RestToPersistenceContext {
                job: PersistenceJobType::Store(sr),
                sender: local_tx,
                span: Span::current(),
            };
            sender.try_send(job)
        }
        PersistenceOperationType::Retrieve => {
            let rr = RetrieveRequest {
                correlation_id,
                table_name: database_name,
                key: key,
            };

            let job = RestToPersistenceContext {
                job: PersistenceJobType::Retrieve(rr),
                sender: local_tx,
                span: Span::current(),
            };
            sender.try_send(job)
        }
        PersistenceOperationType::Delete => {
            let rr = DeleteRequest {
                correlation_id,
                table_name: database_name,
                key: key,
            };

            let job = RestToPersistenceContext {
                job: PersistenceJobType::Delete(rr),
                sender: local_tx,
                span: Span::current(),
            };
            sender.try_send(job)
        }
    };

    if let Err(e) = res {
        warn!("Channel is dead {:?}", e);
        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        *response.body_mut() = Body::empty();
    } else {
        debug!("Waiting for store");
        let response_from_store = local_rx.await;
        debug!("Got result {:?}", response_from_store);
        if let Ok(res) = response_from_store {
            set_http_response(res.status, &mut response);
            if !res.payload.is_empty() {
                *response.body_mut() = Body::from(res.payload);
            }
        } else {
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            *response.body_mut() = Body::empty();
        }
        info!("end {}", msg);
    }
    response
}

async fn sub_unsubscribe_processor(
    is_subscribe: bool,
    csr: ClientSubscribeRequest,
    correlation_id: String,
    from_client_to_backend_channel_sender: &HashMap<String, mpsc::Sender<RestToMessagingContext>>,
    to_client_sender: mpsc::Sender<BackendToRestContext>,
) -> Response<Body> {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NOT_ACCEPTABLE;

    let json: ClientSubscribeRequest = csr;
    let (local_tx, local_rx): (oneshot::Sender<MessagingResult>, oneshot::Receiver<MessagingResult>) = oneshot::channel();
    let endpoint = json.endpoint.clone();
    let sb = SubscribeRequest {
        correlation_id,
        client_interface_type: CustomerInterfaceType::REST,
        client_topic: json.client_topic.clone(),
        endpoint,
        tx: Box::new(to_client_sender.clone()),
    };

    let maybe_channel = find_channel_by_topic(&sb.client_topic, &from_client_to_backend_channel_sender);

    let sender = if let Some(channel) = maybe_channel {
        channel.clone()
    } else {
        set_http_response(BackendStatusCodes::NoTopic("No channel for this topic".to_string()), &mut response);
        return response;
    };
    let ctx = if is_subscribe {
        RestToMessagingContext {
            job: Job::Subscribe(sb),
            sender: local_tx,
            span: Span::current(),
        }
    } else {
        RestToMessagingContext {
            job: Job::Unsubscribe(sb),
            sender: local_tx,
            span: Span::current(),
        }
    };

    debug!("Waiting for broker");
    if let Err(e) = sender.try_send(ctx) {
        warn!("Channel is dead {:?}", e);
        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        *response.body_mut() = Body::empty();
    }

    let response_from_broker = local_rx.await;
    debug!("Got result {:?}", response_from_broker);
    if let Ok(res) = response_from_broker {
        set_http_response(res.status, &mut response);
    } else {
        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        *response.body_mut() = Body::empty();
    }

    response
}

#[instrument(
    name = "CLIENT_HTTP_INCOMING",
    fields(method, uri, correlation_id),
    skip(req, from_client_to_backend_channel_sender, to_client_sender, from_client_to_persistence_sender, from_client_to_si_sender, metric_registry)
)]
pub async fn handler(
    req: Request<Body>,
    from_client_to_backend_channel_sender: HashMap<String, mpsc::Sender<RestToMessagingContext>>,
    to_client_sender: mpsc::Sender<BackendToRestContext>,
    from_client_to_persistence_sender: HashMap<String, mpsc::Sender<RestToPersistenceContext>>,
    from_client_to_si_sender: mpsc::Sender<RestToSIContext>,
    metric_registry: Arc<metric_utils::MetricRegistry>
) -> Result<Response<Body>, hyper::Error> {


    let mut labels = metric_registry.labels.clone();
    let interface = &req.uri().path().to_string();
    labels.push(KeyValue::new("interface", interface.clone()));
		
    let request_start = SystemTime::now();

    metric_registry.http_incoming_counters.request_counter.add(1,&labels);
    
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NOT_ACCEPTABLE;

    let span = tracing_utils::from_http_headers(Span::current(), &req.headers());
    span.record("method", &req.method().as_str());
    span.record("uri", &req.uri().to_string().as_str());

    let headers = req.headers().clone();

    let correlation_id = if let Some(correlation_id) = extract_correlation_id_from_headers(&headers) {
        correlation_id
    } else {
        *response.status_mut() = StatusCode::BAD_REQUEST;
        *response.body_mut() = Body::empty();
        return Ok(response);
    };

    span.record("correlation_id", &correlation_id.as_str());
    info!("Request {:?}", headers);
    
    let response = match (req.method(), req.uri().path()) {
        (&Method::POST, "/pubsub/publish") => {	    
            let whole_body = get_whole_body(req).await;
            let wb = whole_body.clone();
            let wb = String::from_utf8_lossy(&wb);

            let client_topic = extract_topic_from_headers(&headers).unwrap();
            let maybe_channel = find_channel_by_topic(&client_topic, &from_client_to_backend_channel_sender);
            if let Some(channel) = maybe_channel {		
                info!("Publish start {}", wb);		
                let p = PublishRequest {
                    correlation_id,
                    payload: whole_body,
                    client_topic: client_topic.to_owned(),
                };

                let (local_tx, local_rx): (oneshot::Sender<MessagingResult>, oneshot::Receiver<MessagingResult>) = oneshot::channel();
                let ctx = RestToMessagingContext {
                    job: Job::Publish(p),
                    sender: local_tx,
                    span: Span::current(),
                };
                let channel = channel.to_owned();

                if let Err(e) = channel.try_send(ctx) {
                    warn!("Channel is dead {:?}", e);
                    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    *response.body_mut() = Body::empty();
                }

                debug!("Waiting for broker");
                let response_from_broker = local_rx.await;
                debug!("Got result {:?}", response_from_broker);
                if let Ok(res) = response_from_broker {
                    set_http_response(res.status, &mut response);
                } else {
                    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    *response.body_mut() = Body::empty();
                }
                info!("Publish end {}", wb);
                response
            } else {
                set_http_response(BackendStatusCodes::NoTopic("No mapping for this topic".to_string()), &mut response);
                debug!("No mapping for this topic:  {}", &client_topic);
                response
            }
        }

        (&Method::POST, "/pubsub/subscribe") => {
	    if validate_content_type(&headers).is_none() {
		return Ok(response);
	    }
	    let whole_body = get_whole_body(req).await;
	    response = sub_unsubscribe_handler(true, whole_body, correlation_id, &from_client_to_backend_channel_sender, to_client_sender).await;
	    response
        }
	
        (&Method::POST, "/pubsub/unsubscribe") => {
            if validate_content_type(&headers).is_none() {
                return Ok(response);
            }
            let whole_body = get_whole_body(req).await;
            response = sub_unsubscribe_handler(false, whole_body, correlation_id, &from_client_to_backend_channel_sender, to_client_sender).await;
            response
        }

        (&Method::POST, "/persistence/store") => {
            let response = persistence_processor(PersistenceOperationType::Store, correlation_id, &headers, req, &from_client_to_persistence_sender).await;
            response
        }

        (&Method::POST, "/persistence/retrieve") => {
            let response = persistence_processor(PersistenceOperationType::Retrieve, correlation_id, &headers, req, &from_client_to_persistence_sender).await;
            response
        }
        (&Method::POST, "/persistence/delete") => {
            let response = persistence_processor(PersistenceOperationType::Delete, correlation_id, &headers, req, &from_client_to_persistence_sender).await;
            response
        }
        (&Method::POST, path) if path.starts_with("/serviceinvocation/invoke/") => {	    
            if validate_content_type(&headers).is_none() {
                response
            } else {
                service_invocation_processor(correlation_id, path.to_string(), req, from_client_to_si_sender).await
            }
        }
        // The 404 Not Found route...
        _ => {
            warn!("Don't know what to do {} {}", &req.method(), &req.uri());
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            not_found
        }
    };


    bump_http_response_counters(&response.status(),&metric_registry.http_incoming_counters,&labels);
    
    metric_registry.http_incoming_histograms.request_response_time        .record(request_start.elapsed().map_or(0.0, |d| d.as_secs_f64()),&labels);
    info!("{:?}", response);
    Ok(response)
}

async fn send_request(client: Client<HttpConnector<GaiResolver>>, ctx: BackendToRestContext,metric_registry: Arc<metric_utils::MetricRegistry>) {
    let req_params = ctx.request_params;
    let uri = req_params.uri;
    let upper = req_params.method.to_uppercase();
    let method = upper.as_bytes();

    
		
    let request_start = SystemTime::now();

    

    //TODO: this will drump stack trace. probably just an error. otherwise validate url in http_handler
    let uri = uri.parse::<hyper::Uri>().unwrap();

    let mut labels = metric_registry.labels.clone();
    let interface = &uri.path().to_string();
    labels.push(KeyValue::new("interface", interface.clone()));    
    metric_registry.http_outgoing_counters.request_counter.add(1,&labels);
    
    
    let mut builder = Request::builder().method(method).uri(&uri);

    for (k, v) in req_params.headers.iter() {
        let maybe_header = hyper::header::HeaderName::from_bytes(k.as_bytes());
        let maybe_value = hyper::header::HeaderValue::from_bytes(v.as_bytes());
        if let (Ok(header), Ok(value)) = (maybe_header, maybe_value) {
            builder = builder.header(header, value);
        } else {
            if let Some(sender) = ctx.sender {
                let msg = format!("Invalid header {}", k);
                debug!("{}", msg);
                let res = RESTRequestResult {
                    correlation_id: ctx.correlation_id,
                    status: ClientCallStatusCodes::Error(msg.to_string()),
                    response_params: RESTResponseParams { ..Default::default() },
                };

                if let Err(e) = sender.send(res) {
                    warn!("Problem with an internal communication {:?}", e);
                }
            }
            return;
        }
    }
    if let Some((tracing_header_name, tracing_header_value)) = tracing_utils::get_tracing_header() {
        let maybe_header = hyper::header::HeaderName::from_bytes(tracing_header_name.as_bytes());
        let maybe_value = hyper::header::HeaderValue::from_bytes(tracing_header_value.as_bytes());
        if let (Ok(header), Ok(value)) = (maybe_header, maybe_value) {
            builder = builder.header(header, value);
        }
    }

    let req = builder.body(Body::from(req_params.payload)).expect("request builder");

    //    let p = String::from_utf8_lossy(req.as_bytes());
    debug!("HTTP Request {:?}", &uri);

    if let Some(sender) = ctx.sender {
        let maybe_response = client.request(req).await;
        let res = if let Ok(response) = maybe_response {

	    bump_http_response_counters(&response.status(),&metric_registry.http_outgoing_counters,&labels);
	    	    	    
            let status_code = response.status().as_u16();
            let mut parsed_headers = HashMap::new();
            for (header, value) in response.headers().iter() {
                parsed_headers.insert(header.to_string(), String::from(value.to_str().unwrap()));
            }

            if let Ok(body) = hyper::body::to_bytes(response).await {
                RESTRequestResult {
                    correlation_id: ctx.correlation_id,
                    status: ClientCallStatusCodes::Ok("Cool".to_string()),
                    response_params: RESTResponseParams {
                        payload: body.to_vec(),
                        status_code,
                        headers: parsed_headers,
                    },
                }
            } else {
                RESTRequestResult {
                    correlation_id: ctx.correlation_id,
                    status: ClientCallStatusCodes::Error("Something wrong with body".to_string()),
                    response_params: RESTResponseParams {
                        status_code,
                        headers: parsed_headers,
                        ..Default::default()
                    },
                }
            }
        } else {
            RESTRequestResult {
                correlation_id: ctx.correlation_id,
                status: ClientCallStatusCodes::Error(format!("Got error {:?}", maybe_response)),
                response_params: RESTResponseParams { ..Default::default() },
            }
        };
        if let Err(e) = sender.send(res) {
            warn!("Problem with an internal communication {:?}", e);
        }
    } else {
	let maybe_response = client.request(req).await;
	if let Ok(response) = maybe_response{
	    if  response.status().is_server_error() {
		metric_registry.http_outgoing_counters.server_error_counter.add(1,&labels);
	    }
	    
	    if  response.status().is_client_error() {
		metric_registry.http_outgoing_counters.client_error_counter.add(1,&labels);
	    }
	    
	    if  response.status().is_success() {
		metric_registry.http_outgoing_counters.success_counter.add(1,&labels);
	    }

	    
	}else{	    
            debug!("HTTP Response {:?}", maybe_response);
	}
    }
    metric_registry.http_outgoing_histograms.request_response_time
        .record(request_start.elapsed().map_or(0.0, |d| d.as_secs_f64()),&labels);
    
}

pub async fn client_handler(rx: Arc<Mutex<mpsc::Receiver<BackendToRestContext>>>, metric_registry: Arc<metric_utils::MetricRegistry>) {
    let client = hyper::Client::builder().build_http();
    info!("Client done");
    let mut rx = rx.lock().await;    
    while let Some(ctx) = rx.recv().await {
        let client = client.clone();
        let parent_span = ctx.span.clone();
	let metric_registry = metric_registry.clone();	
        let span = info_span!(parent: parent_span, "CLIENT_HTTP_OUTGOING");
        tokio::spawn(async move { send_request(client, ctx,metric_registry).instrument(span).await });
    }
}
