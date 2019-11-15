use crossbeam_channel::{Receiver, Sender, unbounded};
use futures::future;
use http::HeaderValue;
use hyper::{Body, HeaderMap, Method, Request, Response, StatusCode};
use hyper::rt::{Future, Stream};

use super::utils::structs::{InternalMessage, Job, KafkaResult, PublishRequest};

pub type BoxFut = Box<dyn Future<Item=Response<Body>, Error=hyper::Error> + Send>;


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


pub fn handler(req: Request<Body>, sender: Sender<InternalMessage>) -> BoxFut {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NOT_ACCEPTABLE;

    let headers = req.headers();
    info!("Headers {:?}", headers);

    if validate_content_type(headers).is_none() {
        return Box::new(future::ok(response));
    }

    let (parts, body) = req.into_parts();

    info!("Body {:?}", body);
    let url = parts.uri.clone().to_string();
    match (parts.method, parts.uri.path()) {
        (Method::POST, "/publish") => {
            let mapping = body.concat2()
                .map(|chunk| { chunk.to_vec() })
                .map(|payload| {
                    let p = &String::from_utf8_lossy(&payload);
                    info!("Payload is {:?}", &p);
                    PublishRequest { payload: p.to_string(), url: url }
                }).map(move |p| {
                info!("{:?}", p);
                let (local_tx, local_rx): (Sender<KafkaResult>, Receiver<KafkaResult>) = unbounded();
                let job = InternalMessage { job: Job::Publish(p), sender: local_tx.clone() };
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
                response
            });
            return Box::new(mapping);
        }

        (Method::POST, "/subscribe") => {
            let mapping = body.concat2()
                .map(|chunk| { chunk.to_vec() })
                .map(|payload| {
                    let p = &String::from_utf8_lossy(&payload);
                    info!("Payload is {:?}", &p);
                    serde_json::from_str(&p)
                }).map(move |p| {
                match p {
                    Ok(json) => {
                        info!("{:?}", json);
                        let (local_tx, local_rx): (Sender<KafkaResult>, Receiver<KafkaResult>) = unbounded();
                        let job = InternalMessage { job: Job::Subscribe(json), sender: local_tx.clone() };
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
            return Box::new(mapping);
        }
        // The 404 Not Found route...
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    };

    Box::new(future::ok(response))
}