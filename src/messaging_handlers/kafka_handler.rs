use std::collections::HashMap;
use std::sync::Arc;

use futures::future;
use futures::future::FutureExt;
use futures::lock::Mutex;
use futures::stream::StreamExt;
use rdkafka::client::ClientContext;
use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::{CommitMode, Consumer, ConsumerContext, Rebalance};
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::error::KafkaResult;
use rdkafka::message::{Headers, Message};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::TopicPartitionList;
use sled::{Db, IVec};
use tokio::sync::mpsc;
use tokio::task;

use async_trait::async_trait;

use crate::messaging_handlers::Broker;
use crate::utils::config::Kafka;

use super::super::utils::structs;
use super::super::utils::structs::*;

impl ClientContext for CustomContext {}

#[derive(Debug)]
pub struct KafkaBroker {
    pub kafka: Kafka,
    pub db: Db,
    pub rx: Arc<Mutex<mpsc::Receiver<RestToMessagingContext>>>,
    pub tx: Box<HashMap<CustomerInterfaceType, Box<mpsc::Sender<MessagingToRestContext>>>>,
}

impl ConsumerContext for CustomContext {
    fn pre_rebalance(&self, rebalance: &Rebalance) {
        info!("Pre rebalance {:?}", rebalance);
    }

    fn post_rebalance(&self, rebalance: &Rebalance) {
        info!("Post rebalance {:?}", rebalance);
    }

    fn commit_callback(&self, result: KafkaResult<()>, _offsets: &TopicPartitionList) {
        info!("Committing offsets: {:?}", result);
    }
}

type LoggingConsumer = StreamConsumer<CustomContext>;

async fn send_request(db: &Db, tx: &Box<HashMap<CustomerInterfaceType, Box<mpsc::Sender<MessagingToRestContext>>>>, p: Vec<u8>, topic: Vec<u8>) {
    debug!("Processing message  {:?}", p);
    if let Ok(maybe_url) = db.get(topic) {
        if let Some(url) = maybe_url {
            let vec = url.to_vec();
            let subscribe_request: SubscribeRequest = serde_json::from_slice(&vec).unwrap();
            let (s, _r) = futures::channel::oneshot::channel();

            let p = MessagingToRestContext {
                sender: s,
                payload: p.to_vec(),
                uri: subscribe_request.endpoint.url.clone(),
            };

            let mut tx = tx.get(&subscribe_request.client_interface_type).unwrap().clone();
            if let Err(e) = tx.try_send(p) {
                warn!("Error from the client {}", e)
            }

            //    match r.recv() {
            //        Ok(r) => debug!("Response from the client {:?}", r),
            //        Err(e) => warn!("Internal communication error  {}", e)
            //    }
        }
    }
}

impl KafkaBroker {
    async fn kafka_event_handler(&self) {
        let kafka_producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", self.kafka.brokers.get(0).unwrap())
            .set("message.timeout.ms", "5000")
            .create()
            .expect("Can't start broker");

        info!("Kafka running");

        let mut rx = self.rx.lock().await;

        while let Some(job) = rx.next().await {
            let sender = job.sender;
            match job.job {
                Job::Subscribe(value) => {
                    let req = value;
                    info!("New registration  {:?}", req);

                    let maybe_topic = self.kafka.get_consumer_topic_for_client_topic(&req.client_topic);

                    if let Some(topic) = maybe_topic {
                        let s = serde_json::to_string(&req).unwrap();
                        if let Err(e) = self.db.insert(topic, IVec::from(s.as_bytes())) {
                            warn!("Can't store registration {:?}", e);
                            if let Err(e) = sender.send(structs::MessagingResult {
                                status: BackendStatusCodes::Error(e.to_string()),
                            }) {
                                warn!("Can't send response back {:?}", e);
                            }
                        } else {
                            if let Err(e) = sender.send(structs::MessagingResult {
                                status: BackendStatusCodes::Ok("KAFKA is good".to_string()),
                            }) {
                                warn!("Can't send response back {:?}", e);
                            }
                        };
                    } else {
                        warn!("Can't find topic {:?}", req);
                        if let Err(e) = sender.send(structs::MessagingResult {
                            status: BackendStatusCodes::NoTopic("Can't find produce topic".to_string()),
                        }) {
                            warn!("Can't send response back {:?}", e);
                        }
                    }
                }

                Job::Publish(value) => {
                    let req = value;
                    debug!("Kafka plain sending {:?}", req);
                    let maybe_topic = self.kafka.get_producer_topic_for_client_topic(&req.client_topic);
                    let kafka_producer = kafka_producer.clone();
                    if let Some(topic) = maybe_topic {
                        tokio::spawn(async move {
                            let r = FutureRecord::to(topic.as_str()).payload(&req.payload).key("some key");
                            let foo = kafka_producer.send(r, 0).map(move |status| match status {
                                Ok(_) => sender.send(structs::MessagingResult {
                                    status: BackendStatusCodes::Ok("KAFKA is good".to_string()),
                                }),
                                Err(e) => sender.send(structs::MessagingResult {
                                    status: BackendStatusCodes::Error(e.to_string()),
                                }),
                            });

                            foo.await.expect("Should not panic!");
                        });
                        //                            if let Err(e) = foo.await {
                        //                                warn!("hmmm something is very wrong here. it seems that the channel has been closed {:?}", e);
                        //                            }
                    } else {
                        warn!("Can't find topic {:?}", req);
                        if let Err(e) = sender.send(structs::MessagingResult {
                            status: BackendStatusCodes::NoTopic("Can't find subscribe topic".to_string()),
                        }) {
                            warn!("Can't send response back {:?}", e);
                        }
                    }
                }
            }
        }
    }

    async fn kafka_incoming_event_handler(&self) {
        let context = CustomContext;

        let mut consumer_topics = vec![];
        let mut consumer_groups = vec![];
        for ct in self.kafka.consumer_topics.iter() {
            consumer_topics.push(ct.consumer_topic.clone());
            consumer_groups.push(ct.consumer_group.clone());
        }

        let consumer_group = consumer_groups.get(0).unwrap();
        let broker_address = self.kafka.brokers.get(0).unwrap();

        let consumer: LoggingConsumer = ClientConfig::new()
            .set("group.id", consumer_group)
            .set("bootstrap.servers", broker_address)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            //.set("statistics.interval.ms", "30000")
            //.set("auto.offset.reset", "smallest")
            .set_log_level(RDKafkaLogLevel::Warning)
            .create_with_context(context)
            .expect("Consumer creation failed");

        let topics: Vec<&str> = consumer_topics.iter().map(|x| &**x).collect();
        consumer.subscribe(&topics).expect("Can't subscribe to topics");

        let mut message_stream = consumer.start();

        while let Some(message) = message_stream.next().await {
            match message {
                Err(e) => warn!("Kafka error: {}", e),
                Ok(m) => {
                    let payload = match m.payload_view::<str>() {
                        None => "",
                        Some(Ok(s)) => s,
                        Some(Err(e)) => {
                            warn!("Error while deserializing message payload: {:?}", e);
                            ""
                        }
                    };
                    debug!(
                        "key: '{:?}', payload: '{}', topic: {}, partition: {}, offset: {}, timestamp: {:?}",
                        m.key(),
                        payload,
                        m.topic(),
                        m.partition(),
                        m.offset(),
                        m.timestamp()
                    );
                    //                    if let Some(headers) = m.headers() {
                    //                        for i in 0..headers.count() {
                    //                            let header = headers.get(i).unwrap();
                    //                            info!("  Header {:#?}: {:?}", header.0, header.1);
                    //                        }
                    //                    }
                    let t = Vec::from(m.topic());
                    let vec = Vec::from(payload);
                    let db = self.db.clone();
                    let tx = self.tx.clone();
                    //                    consumer.commit_message(&m, CommitMode::Async).unwrap();
                    tokio::spawn(async move {
                        send_request(&db, &tx, vec, t).await;
                    });
                }
            };
        }
    }

    //    async fn kafka_incoming_event_handler(&self) {
    //        let context = CustomContext;
    //
    //        let mut consumer_topics = vec![];
    //        let mut consumer_groups = vec![];
    //        for ct in self.kafka.consumer_topics.iter() {
    //            consumer_topics.push(ct.consumer_topic.clone());
    //            consumer_groups.push(ct.consumer_group.clone());
    //        }
    //
    //        let consumer_group = consumer_groups.get(0).unwrap().clone();
    //        let broker_address = self.kafka.brokers.get(0).unwrap().clone();
    //
    //        //        let consumer: LoggingConsumer = ClientConfig::new()
    //
    //        let db = self.db.clone();
    //        let tx = self.tx.clone();
    //
    //        let join = task::spawn_blocking(move || {
    //            let consumer: rdkafka::consumer::BaseConsumer<CustomContext> = ClientConfig::new()
    //                .set("group.id", &consumer_group)
    //                .set("bootstrap.servers", &broker_address)
    //                .set("enable.partition.eof", "false")
    //                .set("session.timeout.ms", "6000")
    //                .set("enable.auto.commit", "true")
    //                //.set("statistics.interval.ms", "30000")
    //                //.set("auto.offset.reset", "smallest")
    //                .set_log_level(RDKafkaLogLevel::Warning)
    //                .create_with_context(context)
    //                .expect("Consumer creation failed");
    //
    //            let topics: Vec<&str> = consumer_topics.iter().map(|x| &**x).collect();
    //            consumer.subscribe(&topics).expect("Can't subscribe to topics");
    //
    //            for message in consumer.iter() {
    //                let a = future::ready(match message {
    //                    Err(e) => warn!("Kafka error: {}", e),
    //                    Ok(m) => {
    //                        let payload = match m.payload_view::<str>() {
    //                            None => "",
    //                            Some(Ok(s)) => s,
    //                            Some(Err(e)) => {
    //                                warn!("Error while deserializing message payload: {:?}", e);
    //                                ""
    //                            }
    //                        };
    //                        //                        info!(
    //                        //                            "key: '{:?}', payload: '{}', topic: {}, partition: {}, offset: {}, timestamp: {:?}",
    //                        //                            m.key(),
    //                        //                            payload,
    //                        //                            m.topic(),
    //                        //                            m.partition(),
    //                        //                            m.offset(),
    //                        //                            m.timestamp()
    //                        //                        );
    //                        //                        if let Some(headers) = m.headers() {
    //                        //                            for i in 0..headers.count() {
    //                        //                                let header = headers.get(i).unwrap();
    //                        //                                info!("  Header {:#?}: {:?}", header.0, header.1);
    //                        //                            }
    //                        //                        }
    //                        let t = Vec::from(m.topic());
    //                        let vec = Vec::from(payload);
    //
    //                        //                    consumer.commit_message(&m, CommitMode::Async).unwrap();
    //
    //                        send_request(&db, &tx, vec, t);
    //                    }
    //                });
    //                tokio::spawn(a);
    //            }
    //        });
    //        let _res = join.await;
    //    }
}

#[async_trait]
impl Broker for KafkaBroker {
    async fn configure_broker(&self) {
        info!("Configuring KAFKA broker {:?} ", self);
        let f1 = async { self.kafka_incoming_event_handler().await };
        let f2 = async { self.kafka_event_handler().await };
        let (_r1, _r2) = futures::join!(f1, f2);
    }
}
