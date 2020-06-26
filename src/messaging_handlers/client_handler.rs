use super::super::utils::config::ClientTopicsConfiguration;
use super::super::utils::structs;
use super::super::utils::structs::*;
use async_trait::async_trait;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{oneshot::Sender, Mutex};

type Subscriptions = HashMap<String, Box<Vec<SubscribeRequest>>>;

#[async_trait]
pub trait ClientHandler {
    fn get_subscriptions(&self) -> Arc<Mutex<Box<Subscriptions>>>;
    fn get_configuration(&self) -> Box<dyn ClientTopicsConfiguration + Send>;
    fn get_type(&self) -> String;

    async fn subscribe(&self, req: SubscribeRequest, sender: Sender<MessagingResult>) {
        let subscriptions = self.get_subscriptions();
        let nats = self.get_configuration();
        let broker_type = self.get_type();
        info!("Subscribe {}", req);
        let client_topic = req.client_topic.clone();
        let maybe_topic = nats.get_consumer_topic_for_client_topic(&client_topic);

        if let Some(topic) = maybe_topic {
            let mut subscriptions = subscriptions.lock().await;
            if let Some(subscriptions_for_topic) = subscriptions.get_mut(&topic) {
                if subscriptions_for_topic.binary_search(&req).is_err() {
                    debug!("Adding subscription {}", req);
                    subscriptions_for_topic.push(req.clone());
                    if let Err(e) = sender.send(structs::MessagingResult {
                        correlation_id: req.correlation_id,
                        status: BackendStatusCodes::Ok(format!("{} has {} susbscriptions for topic {}", broker_type, subscriptions_for_topic.len(), &client_topic)),
                    }) {
                        warn!("Can't send response back {:?}", e);
                    }
                } else {
                    debug!("Subscription exists for {:?}", req);
                    if let Err(e) = sender.send(structs::MessagingResult {
                        correlation_id: req.correlation_id,
                        status: BackendStatusCodes::NoTopic(format!("{}: Duplicate subscription for topic {}", &broker_type, &client_topic)),
                    }) {
                        warn!("Can't send response back {:?}", e);
                    }
                }
            } else {
                info!("Can't find subscriptions {} adding new one", req);
                subscriptions.insert(topic.clone(), Box::new(vec![req.clone()]));
                if let Err(e) = sender.send(structs::MessagingResult {
                    correlation_id: req.correlation_id,
                    status: BackendStatusCodes::Ok(format!("{} has one susbscription for topic {}", &broker_type, &client_topic)),
                }) {
                    warn!("Can't send response back {:?}", e);
                }
            }
        } else {
            warn!("Can't find topic {:?}", req);
            if let Err(e) = sender.send(structs::MessagingResult {
                correlation_id: req.correlation_id,
                status: BackendStatusCodes::NoTopic(format!("{} has no topic {}", &broker_type, &client_topic)),
            }) {
                warn!("Can't send response back {:?}", e);
            }
        }
    }

    async fn unsubscribe(&self, req: SubscribeRequest, sender: Sender<MessagingResult>) {
        let subscriptions = self.get_subscriptions();
        let nats = self.get_configuration();
        let broker_type = self.get_type();
        info!("Unsubscribe {}", req);
        let client_topic = req.client_topic.clone();
        let maybe_topic = nats.get_consumer_topic_for_client_topic(&client_topic);

        if let Some(topic) = maybe_topic {
            let mut remove_topic = false;
            let mut subscriptions = subscriptions.lock().await;
            if let Some(subscriptions_for_topic) = subscriptions.get_mut(&topic) {
                if let Ok(index) = subscriptions_for_topic.binary_search(&req) {
                    debug!("Subscription exists for {}", req);
                    subscriptions_for_topic.remove(index);
                    if subscriptions_for_topic.len() == 0 {
                        remove_topic = true;
                        debug!("All subscriptions removed for {}", topic);
                    }

                    if let Err(e) = sender.send(structs::MessagingResult {
                        correlation_id: req.correlation_id,
                        status: BackendStatusCodes::Ok(format!("{} has {} susbscriptions for topic {}", &broker_type, subscriptions_for_topic.len(), &client_topic)),
                    }) {
                        warn!("Can't send response back {:?}", e);
                    }
                } else {
                    debug!("No subscriptions  {}", req);
                    if let Err(e) = sender.send(structs::MessagingResult {
                        correlation_id: req.correlation_id,
                        status: BackendStatusCodes::NoTopic(format!("{} has no subscription for topic {}", &broker_type, &client_topic)),
                    }) {
                        warn!("Can't send response back {:?}", e);
                    }
                }
            }
            if remove_topic {
                subscriptions.remove(&topic);
            }
        } else {
            warn!("Can't find topic {}", req);
            if let Err(e) = sender.send(structs::MessagingResult {
                correlation_id: req.correlation_id,
                status: BackendStatusCodes::NoTopic(format!("{} has no topic {}", &broker_type, &client_topic)),
            }) {
                warn!("Can't send response back {:?}", e);
            }
        }
    }
}
