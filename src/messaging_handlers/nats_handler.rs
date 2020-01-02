use std::collections::HashMap;
use std::sync::Arc;

use futures::lock::Mutex;
use futures::stream::StreamExt;
use nats::*;
use tokio::sync::mpsc;
use tokio::task;

use async_trait::async_trait;

use crate::messaging_handlers::Broker;
use crate::utils::config::Nats;

use super::super::utils::structs;
use super::super::utils::structs::*;

#[derive(Debug)]
pub struct NatsBroker {
    pub nats: Nats,
    pub rx: Arc<Mutex<mpsc::Receiver<RestToMessagingContext>>>,
    pub tx: Box<HashMap<CustomerInterfaceType, Box<mpsc::Sender<MessagingToRestContext>>>>,
    pub subscriptions: Arc<Mutex<Box<HashMap<String, Box<Vec<SubscribeRequest>>>>>>,
}

fn send_request(mut subscriptions:  Box<Vec<SubscribeRequest>>, p: Vec<u8>, topic: String) {
    debug!("Processing message  {:?}", p);
    
    for subscription in subscriptions.iter_mut(){	
	let (s, _r) = futures::channel::oneshot::channel();
	debug!("Processing subscription  {:?}", subscription);
	let p = MessagingToRestContext {
	    sender: s,
	    payload: p.to_vec(),
	    uri: subscription.endpoint.url.clone(),
        };
	if let Err(e) = subscription.tx.try_send(p){
	    warn!("Unable to send. Channel could be closed {}", e)
	}
    }
}

impl NatsBroker {    
    async fn nats_event_handler(&self, mut nats: Client) {
        let mut rx = self.rx.lock().await;
        while let Some(job) = rx.next().await {
            let sender = job.sender;
            match job.job {
                Job::Subscribe(value) => {
                    let req = value;
                    info!("New registration  {:?}", req);
		    let maybe_topic = self.nats.get_consumer_topic_for_client_topic(&req.client_topic);

                    if let Some(topic) = maybe_topic {			
			let mut subscriptions = self.subscriptions.lock().await;		
			if let Some(subscriptions_for_topic) = subscriptions.get_mut(&topic){
			    if let Ok(index) = subscriptions_for_topic.binary_search(&req){
				let old_subscription = subscriptions_for_topic.remove(index);
				debug!("Old subscription {:?}",old_subscription);
			    }
			    subscriptions_for_topic.push(req.clone());				
			    if let Err(e) = sender.send(structs::MessagingResult {
				status: BackendStatusCodes::Ok(format!("NATS has {} susbscriptions for topic {}",subscriptions_for_topic.len(),topic.clone()).to_string()),
                            }) {
				warn!("Can't send response back {:?}", e);
                            }
			}else{
			    warn!("Can't find subscriptions {:?} adding new one", req);
			    subscriptions.insert(topic.clone(), Box::new(vec![req.clone()]));
                            if let Err(e) = sender.send(structs::MessagingResult {
				status: BackendStatusCodes::Ok(format!("NATS has one susbscription for topic {}",topic.clone()).to_string()),
                            }) {
				warn!("Can't send response back {:?}", e);
                            }
			}						
                    } else {
                        warn!("Can't find topic {:?}", req);
                        if let Err(e) = sender.send(structs::MessagingResult {
                            status: BackendStatusCodes::NoTopic("Can't find subscribe topic".to_string()),
                        }) {
                            warn!("Can't send response back {:?}", e);
                        }
                    }

                }

                Job::Publish(value) => {
                    let req = value;
                    let maybe_topic = self.nats.get_producer_topic_for_client_topic(&req.client_topic);
                    if let Some(topic) = maybe_topic {
                        let foo = nats.publish(&topic, &req.payload);
                        match foo {
                            Ok(_) => {
                                sender.send(structs::MessagingResult {
                                    status: BackendStatusCodes::Ok("NATS is good".to_string()),
                                });
                            }
                            Err(e) => {
                                sender.send(structs::MessagingResult {
                                    status: BackendStatusCodes::Error(e.to_string()),
                                });
                            }
                        }
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



    
    async fn nats_incoming_event_handler(&self, client: Option<Client>) {
        
        let tx = self.tx.clone();

	if client.is_none(){
	    return
	}
	let mut client = client.unwrap();

	let subscriptions = self.subscriptions.clone();
        let join = task::spawn_blocking(move || {
            info!("Waiting for events ");
            for event in client.events() {
		let topic = event.subject;
		let mut subs = Box::new(Vec::new());
		
		let mut hasLock = false;
		while !hasLock{
		    if let Some(mut subscriptions) = subscriptions.try_lock(){
			if let Some(subscriptions) = subscriptions.get_mut(&topic){			
			    subs = subscriptions.clone();
			}
			hasLock = true;
		    }			
		}		
		send_request(subs, event.msg,topic);		
            }
        });
        let _res = join.await;
    }
}




#[async_trait]
impl Broker for NatsBroker {
    async fn configure_broker(&self) {
        info!("Configuring NATS broker {:?} ", self);
        let cluster = self.nats.brokers.clone();
        let mut incoming_client = Option::<nats::Client>::None;
        let outgoing_client = nats::Client::new(cluster.clone()).unwrap();
	
	if !self.nats.consumer_topics.is_empty(){
	    let mut consumer_topics = vec![];
            let mut consumer_groups = vec![];
            for ct in self.nats.consumer_topics.iter() {
		consumer_topics.push(ct.consumer_topic.clone());
		consumer_groups.push(ct.consumer_group.clone());
            }
	    let consumer_group = consumer_groups.get(0).unwrap();
            let consumer_topic = consumer_topics.get(0).unwrap();
	    let mut client = nats::Client::new(cluster).unwrap();
	    client.set_name(&consumer_group);
            client.subscribe(consumer_topic.as_str(), Some(&consumer_group)).unwrap();
	    incoming_client = Some(client);
	}else{
	    info!("No consumers configured");
	};
		
        let mut producer_topics = vec![];
        for pt in self.nats.producer_topics.iter() {
            producer_topics.push(pt.producer_topic.clone());
        }

        info!("NATS subscribed and connected");

	let f1 = async { self.nats_incoming_event_handler(incoming_client).await };
        let f2 = async { self.nats_event_handler(outgoing_client).await };
        let (_r1, _r2) = futures::join!(f1, f2);
    }
}
