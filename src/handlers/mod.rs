use std::borrow::Borrow;

use crossbeam_channel::{Receiver, Sender, unbounded};
use futures::future;
use http::HeaderValue;
use hyper::{Body, Client, HeaderMap, Method, Request, Response, StatusCode, Uri};
use hyper::body::Payload;
use hyper::client::HttpConnector;
use hyper::rt::{Future, Stream};
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::Message;
use rdkafka::message::ToBytes;
use rdkafka::producer::{FutureProducer, FutureRecord};
use sled::{Db, IVec};

use structs::{Job, KafkaResult, PublishRequest};

pub mod structs;

#[derive(Debug)]
pub struct InternalMessage {
    job: Job,
    sender: Sender<KafkaResult>,
}


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
                    ;
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
                            ;
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

pub fn kafka_event_handler(rx: &Receiver<InternalMessage>, kafka_producer: &FutureProducer, publish_topic: &str, subscribe_topic: &str, db: &Db) {
    let job = rx.recv().unwrap();

    let sender = job.sender;
    match job.job {
        Job::Subscribe(value) => {
            let req = value;
            info!("New registration  {:?}", req);
            if let Err(e) = db.insert(subscribe_topic, IVec::from(req.endpoint.url.as_bytes())) {
                warn!("Can't store registration {:?}", e);
                if let Err(e) = sender.send(KafkaResult { result: e.to_string() }) {
                    warn!("Can't send response back {:?}", e);
                }
            } else {
                if let Err(e) = sender.send(KafkaResult { result: "All is good".to_string() }) {
                    warn!("Can't send response back {:?}", e);
                }
            };
        }

        Job::Publish(value) => {
            let req = value;
            info!("Kafka plain sending {:?}", req);
            let r = FutureRecord::to(publish_topic).payload(ToBytes::to_bytes(&req.payload))
                .key("some key".to_bytes());
            let foo = kafka_producer.send(r, 0).map(move |status| {
                match status {
                    Ok(_) => {
                        sender.send(KafkaResult { result: "All is good".to_string() })
                    }
                    Err(e) => {
                        sender.send(KafkaResult { result: e.0.to_string() })
                    }
                }
            });
            if let Err(_) = foo.wait() {
                warn!("hmmm something is very wrong here. it seems that the channel has been closed");
            }
        }
    }
}

pub fn kafka_incoming_event_handler(consumer: &StreamConsumer<structs::CustomContext>, client: &Client<HttpConnector, Body>, db: &Db) {
    let stream = consumer.start();
    for message in stream.wait() {
        let f = {
            match message {
                Err(_) => {
                    warn!("Error while reading from stream.");
                }
                Ok(Ok(m)) => {
                    let payload = match m.payload_view::<str>() {
                        None => "",
                        Some(Ok(s)) => s,
                        Some(Err(e)) => {
                            warn!("Error while deserializing message payload: {:?}", e);
                            ""
                        }
                    };

                    let mut uri: String = String::from("");
                    if let Ok(maybe_url) = db.get(m.topic()) {
                        if let Some(url) = maybe_url {
                            let vec = url.to_vec();
                            uri = String::from_utf8_lossy(vec.borrow()).to_string();
                        }
                    }

                    let (s, r) = futures::sync::oneshot::channel::<u16>();


                    let mut postreq = Request::new(Body::from(payload.to_owned()));
                    *postreq.method_mut() = Method::POST;
                    *postreq.uri_mut() = Uri::from(uri.parse().unwrap_or_default());
                    postreq.headers_mut().insert(
                        hyper::header::CONTENT_TYPE,
                        HeaderValue::from_static("application/json"),
                    );
                    let cl = postreq.body_mut().content_length().unwrap().to_string();
                    postreq.headers_mut().insert(
                        hyper::header::CONTENT_LENGTH,
                        HeaderValue::from_str(&cl).unwrap(),
                    );

                    let post = client.request(postreq).and_then(|res| {
                        let status = res.status();
                        info!("POST: {}", res.status());
                        if let Err(e) = s.send(status.as_u16()) {
                            warn!("Problem with an internal communication {:?}", e);
                        }

                        res.into_body().concat2()
                    }).map(|_| {
                        info!("Done.");
                    }).map_err(|err| {
                        warn!("Error {}", err);
                    });
                    hyper::rt::run(post);
                    let result = r.map(|f| {
                        if f == 200 {
                            consumer.commit_message(&m, CommitMode::Async).unwrap();
                        }
                    }).wait();
                    if let Err(e) = result {
                        warn!("Kafka error when commiting messages: {}", e);
                    }
                }
                Ok(Err(e)) => {
                    warn!("Kafka error: {}", e);
                }
            }
            future::ok(())
        };
        info!("got message. about to spawn");
        hyper::rt::run(f);
    };
}


