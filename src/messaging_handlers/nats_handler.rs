use bytes::Bytes;
use futures::stream::StreamExt;
use nats::*;
use std::{collections::HashMap, sync::Arc, thread};

mod nats_msg_wrapper {
    include!(concat!(env!("OUT_DIR"), "/nats_msg_wrapper.rs"));
}
use nats_msg_wrapper::NatsMessageWrapper;

use tokio::{
    sync::{mpsc, Mutex},
    task,
};

use crate::messaging_handlers::{client_handler::ClientHandler, Broker};

use crate::utils::{
    config::{ClientTopicsConfiguration, Nats},
    structs::*,
};
use async_trait::async_trait;
use prost::Message;

type Subscriptions = HashMap<String, Box<Vec<SubscribeRequest>>>;
use crate::utils::tracing_utils;
use tracing::{info_span, Span};
use tracing_futures::Instrument;

#[derive(Debug)]
pub struct NatsBroker {
    nats: Nats,
    rx: Arc<Mutex<mpsc::Receiver<RestToMessagingContext>>>,
    subscriptions: Arc<Mutex<Box<Subscriptions>>>,
}

fn send_request(subscriptions: &mut Vec<SubscribeRequest>, p: Vec<u8>) {
    for subscription in subscriptions.iter_mut() {
        let mut got_sent = false;
        while !got_sent {
            let mrc = BackendToRestContext {
                span: Span::current(),
                correlation_id: subscription.to_string(),
                sender: None,
                request_params: RESTRequestParams {
                    payload: p.to_owned(),
                    method: "POST".to_string(),
                    uri: subscription.endpoint.url.clone(),
                    ..Default::default()
                },
            };

            match subscription.tx.try_send(mrc) {
                Ok(_) => {
                    got_sent = true;
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    got_sent = true;
                    warn!("Unable to send {}. Channel is closed", subscription);
                }
                Err(mpsc::error::TrySendError::Full(_)) => {
                    warn!("Unable to send {}. Channel is full", subscription);
                    thread::yield_now();
                }
            }
        }
    }
}

#[async_trait]
impl ClientHandler for NatsBroker {
    fn get_configuration(&self) -> Box<dyn ClientTopicsConfiguration + Send> {
        Box::new(self.nats.clone())
    }
    fn get_subscriptions(&self) -> Arc<Mutex<Box<Subscriptions>>> {
        self.subscriptions.clone()
    }
    fn get_type(&self) -> String {
        "Nats".to_string()
    }
}

impl NatsBroker {
    pub fn new(config: Nats, rx: Arc<Mutex<mpsc::Receiver<RestToMessagingContext>>>) -> Self {
        NatsBroker {
            nats: config,
            rx,
            subscriptions: Arc::new(Mutex::new(Box::new(HashMap::new()))),
        }
    }

    async fn nats_event_handler(&self, nats: &Connection) {
        let mut rx = self.rx.lock().await;
        while let Some(ctx) = rx.next().await {
            let parent_span = ctx.span;
            let span = info_span!(parent: &parent_span, "NATS_OUTGOING");
            let sender = ctx.sender;

            match ctx.job {
                Job::Subscribe(value) => {
                    self.subscribe(value, sender).instrument(span).await;
                }

                Job::Unsubscribe(value) => {
                    self.unsubscribe(value, sender).instrument(span).await;
                }

                Job::Publish(value) => {
                    let req = value;

                    async {
                        debug!("Publish {}", &req.correlation_id);

                        let maybe_topic = self.nats.get_producer_topic_for_client_topic(&req.client_topic);

                        let mut headers = HashMap::new();
                        if let Some((header_name, header_value)) = tracing_utils::get_tracing_header() {
                            headers.insert(header_name.to_string(), header_value);
                        }
                        if let Some(topic) = maybe_topic {
                            let wrapper = NatsMessageWrapper { headers, payload: req.payload };
                            let mut p = vec![];
                            let res = wrapper.encode(&mut p);
                            if let Ok(_) = res {
                                let nats_publish = nats.publish(&topic, &p);
                                match nats_publish {
                                    Ok(_) => {
                                        let res = sender.send(MessagingResult {
                                            correlation_id: req.correlation_id,
                                            status: BackendStatusCodes::Ok("NATS is good".to_string()),
                                        });
                                        if res.is_err() {
                                            warn!("{:?}", res);
                                        }
                                    }
                                    Err(e) => {
                                        let res = sender.send(MessagingResult {
                                            correlation_id: req.correlation_id,
                                            status: BackendStatusCodes::Error(e.to_string()),
                                        });
                                        if res.is_err() {
                                            warn!("{:?}", res);
                                        }
                                    }
                                }
                            } else {
                                warn!("Problem with encoding NATS payload {:?}", res)
                            }
                        } else {
                            warn!("Can't find topic {:?}", req);
                            let res = sender.send(MessagingResult {
                                correlation_id: req.correlation_id,
                                status: BackendStatusCodes::NoTopic("Can't find subscribe topic".to_string()),
                            });
                            if res.is_err() {
                                warn!("Can't send response back {:?}", res);
                            }
                        }
                    }
                    .instrument(span)
                    .await;
                }
            }
        }
    }

    async fn nats_incoming_event_handler(&self, nats: &Connection) {
        let subscriptions = self.subscriptions.clone();
        let mut tasks = vec![];
        if !self.nats.consumer_topics.is_empty() {
            for ct in self.nats.consumer_topics.iter() {
                let topic = ct.consumer_topic.clone();
                let group = ct.consumer_group.clone();
                debug!("Subscribing to topic {:?}", &ct);
                let subscriptions = subscriptions.clone();

                if let Ok(subscription) = nats.queue_subscribe(&topic, &group) {
                    let subscription = subscription.clone();
                    let job = task::spawn_blocking(move || {
                        info!("Waiting for events {:?}", subscription);
                        for msg in subscription.messages() {
                            let topic = msg.subject;
                            let maybe_msg = NatsMessageWrapper::decode(Bytes::from(msg.data));
                            let span = info_span!("NATS_INCOMING", topic = &topic.as_str());
                            if let Ok(wrapper) = &maybe_msg {
                                let span = tracing_utils::from_map(span, &wrapper.headers);
                                let _sp = span.enter();
                                let mut subs = Box::new(Vec::new());
                                let mut has_lock = false;
                                while !has_lock {
                                    if let Ok(mut subscriptions) = subscriptions.try_lock() {
                                        if let Some(subscriptions) = subscriptions.get_mut(&topic) {
                                            subs = subscriptions.clone();
                                        }
                                        has_lock = true;
                                    }
                                }
                                send_request(&mut subs, wrapper.payload.to_owned());
                            } else {
                                warn!("Unable to decode NATS message {:?}", maybe_msg);
                            };
                        }
                    });
                    tasks.push(job);
                } else {
                    warn!("Can't subscribe ");
                }
            }
        } else {
            info!("No consumers configured");
        };
        futures::future::join_all(tasks).await;
    }
}

#[async_trait]
impl Broker for NatsBroker {
    async fn configure_broker(&self) {
        info!("Configuring NATS broker {:?} ", self);
        let cluster = self.nats.brokers.get(0).unwrap().clone();
        let nc = nats::connect(&cluster).unwrap();

        let mut producer_topics = vec![];
        for pt in self.nats.producer_topics.iter() {
            producer_topics.push(pt.producer_topic.clone());
        }

        info!("NATS subscribed and connected");

        let f1 = async { self.nats_incoming_event_handler(&nc).await };
        let f2 = async { self.nats_event_handler(&nc).await };
        let (_r1, _r2) = futures::join!(f1, f2);
    }
}
