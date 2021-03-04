use std::{collections::HashMap, sync::Arc, time::Duration};

use async_trait::async_trait;
use futures::future::FutureExt;

use futures::stream::StreamExt;
use rdkafka::{
    client::ClientContext,
    config::{ClientConfig, RDKafkaLogLevel},
    consumer::{stream_consumer::StreamConsumer, Consumer, ConsumerContext, Rebalance},
    error::KafkaResult,
    message::{Headers, Message, OwnedHeaders},
    producer::{FutureProducer, FutureRecord},
    TopicPartitionList,
};
use tokio::sync::{mpsc, Mutex};

use crate::messaging_handlers::Broker;

use crate::utils::{
    config::{ClientTopicsConfiguration, Kafka},
    structs::*,
};

use crate::messaging_handlers::client_handler::ClientHandler;

use crate::tracing_utils;
use tracing::{info_span, Span};
use tracing_futures::Instrument;

type Subscriptions = HashMap<String, Box<Vec<SubscribeRequest>>>;

impl ClientContext for CustomContext {}

#[derive(Debug)]
pub struct KafkaBroker {
    kafka: Kafka,
    rx: Arc<Mutex<mpsc::Receiver<RestToMessagingContext>>>,
    subscriptions: Arc<Mutex<Box<Subscriptions>>>,
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

async fn send_request(subscriptions: &mut Vec<SubscribeRequest>, p: Vec<u8>) {
    let msg = String::from_utf8_lossy(&p);
    debug!("Processing message {} {:?}", subscriptions.len(), msg);

    for subscription in subscriptions.iter_mut() {
        debug!("Processing subscription {}", subscription);
        let mrc = BackendToRestContext {
            span: Span::current(),
            correlation_id: subscription.to_string(),
            sender: None,
            request_params: RESTRequestParams {
                payload: p.to_vec(),
                method: "POST".to_string(),
                uri: subscription.endpoint.url.clone(),
                ..Default::default()
            },
        };

        match subscription.tx.send(mrc).await {
            Ok(_) => {
                debug!("Message sent {:?}", msg);
            }

            Err(mpsc::error::SendError(_)) => {
                warn!("Unable to send {}. Channel is closed", subscription);
            }
        }
    }
}

#[async_trait]
impl ClientHandler for KafkaBroker {
    fn get_configuration(&self) -> Box<dyn ClientTopicsConfiguration + Send> {
        Box::new(self.kafka.clone())
    }
    fn get_subscriptions(&self) -> Arc<Mutex<Box<Subscriptions>>> {
        self.subscriptions.clone()
    }
    fn get_type(&self) -> String {
        "Kafka".to_string()
    }
}

impl KafkaBroker {
    pub fn new(config: Kafka, rx: Arc<Mutex<mpsc::Receiver<RestToMessagingContext>>>) -> Self {
        KafkaBroker {
            kafka: config,
            rx,
            subscriptions: Arc::new(Mutex::new(Box::new(HashMap::new()))),
        }
    }

    async fn kafka_event_handler(&self) {
        let kafka_producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", self.kafka.brokers.get(0).unwrap())
            .set("message.timeout.ms", "5000")
            .create()
            .expect("Can't start broker");

        info!("Kafka running");
        let mut rx = self.rx.lock().await;
        while let Some(ctx) = rx.recv().await {
            let parent_span = ctx.span;
            let span = info_span!(parent: &parent_span, "KAFKA_OUTGOING");
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
                    let maybe_topic = self.kafka.get_producer_topic_for_client_topic(&req.client_topic);
                    let kafka_producer = kafka_producer.clone();
                    if let Some(topic) = maybe_topic {
                        tokio::spawn(
                            async move {
                                debug!("Publish {}", req);
                                let headers = if let Some((header_name, header_value)) = tracing_utils::get_tracing_header() {
                                    OwnedHeaders::new().add(header_name, header_value.as_bytes())
                                } else {
                                    OwnedHeaders::new()
                                };
                                let payload = req.payload.clone();
                                let r = FutureRecord::to(topic.as_str()).payload(&payload).key("some key").headers(headers);
                                let kafka_send = kafka_producer.send(r, Duration::from_millis(0)).map(move |status| match status {
                                    Ok(_) => sender.send(MessagingResult {
                                        correlation_id: req.correlation_id,
                                        status: BackendStatusCodes::Ok("KAFKA is good".to_string()),
                                    }),
                                    Err((e, _m)) => sender.send(MessagingResult {
                                        correlation_id: req.correlation_id,
                                        status: BackendStatusCodes::Error(e.to_string()),
                                    }),
                                });

                                kafka_send.await.expect("Should not panic!");
                            }
                            .instrument(span),
                        );
                    //                            if let Err(e) = foo.await {
                    //                                warn!("hmmm something is very wrong here. it seems that the channel has been closed {:?}", e);
                    //                            }
                    } else {
                        warn!("Can't find topic {}", req);
                        let res = sender.send(MessagingResult {
                            correlation_id: req.correlation_id,
                            status: BackendStatusCodes::NoTopic("Can't find subscribe topic".to_string()),
                        });
                        if res.is_err() {
                            warn!("Can't send response back {:?}", res);
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

        if self.kafka.consumer_topics.is_empty() {
            info!("No consumers configured, bye");
            return;
        }
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
        {
            let subscriptions = consumer.subscription().expect("Can't subscribe to topics");
            info!("Subsciptions {:?}", subscriptions);
            let subscriptions = consumer.assignment().expect("Can't subscribe to topics");
            info!("Subsciptions {:?}", subscriptions);
        }

        let mut message_stream = consumer.stream();
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
                        "Message from kafka key: '{:?}', payload: '{}', topic: {}, partition: {}, offset: {}, timestamp: {:?}",
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
                    //                            info!("  Header {:#?}: {:?}", header.0, <header.1);
                    //                        }
                    //                    }
                    let t = String::from(m.topic());
                    let vec = Vec::from(payload);
                    let mut subscriptions = self.subscriptions.lock().await;
                    if let Some(mut subs) = subscriptions.get_mut(&t) {
                        if subs.len() != 0 {
                            let mut span = info_span!("KAFKA_INCOMING");
                            if let Some(headers) = m.headers() {
                                for i in 0..headers.count() {
                                    if let Some(("traceparent", value)) = headers.get(i) {
                                        span = tracing_utils::from_bytes(span, value);
                                    }
                                }
                            }
                            send_request(&mut subs, vec).instrument(span).await;
                        } else {
                            warn!("No subscriptions for {} {}", t, String::from_utf8_lossy(&vec));
                        }
                    }
                }
            };
        }
    }
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
