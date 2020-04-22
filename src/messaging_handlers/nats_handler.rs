use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use futures::lock::Mutex;
use futures::stream::StreamExt;
use nats::*;
use tokio::sync::mpsc;
use tokio::task;
    
use async_trait::async_trait;
use crate::messaging_handlers::Broker;
use crate::messaging_handlers::client_handler::ClientHandler;
use crate::utils::config::Nats;


use super::super::utils::structs;
use super::super::utils::structs::*;
use super::super::utils::config::ClientTopicsConfiguration;


type Subscriptions = HashMap<String, Box<Vec<SubscribeRequest>>>;

#[derive(Debug)]
pub struct NatsBroker{
    nats: Nats,
    rx: Arc<Mutex<mpsc::Receiver<RestToMessagingContext>>>,
    subscriptions: Arc<Mutex<Box<Subscriptions>>>,
}

fn send_request(subscriptions:  &mut Vec<SubscribeRequest>, p: Vec<u8>) {
    let msg = String::from_utf8_lossy(&p);
    debug!("Processing message {} {}", subscriptions.len(), msg);
    
    for subscription in subscriptions.iter_mut(){		
	let mut got_sent = false;
	while !got_sent{

	    let mrc = BackendToRestContext {
		correlation_id: subscription.to_string(),
		sender: None,
		request_params: RESTRequestParams{		
		    payload: p.to_vec(),
		    method: "POST".to_string(),
		    uri: subscription.endpoint.url.clone(),
		    ..Default::default()
		}
            };
	    
	    match subscription.tx.try_send(mrc){
		Ok(_) => {
		    debug!("Message sent {}",msg);
		    got_sent = true;
		},	    
		Err(mpsc::error::TrySendError::Closed(_)) => {
		    got_sent = true;
		    warn!("Unable to send {}. Channel is closed", subscription);
		},
		Err(mpsc::error::TrySendError::Full(_)) => {
		    warn!("Unable to send {} {} . Channel is full", subscription,msg);		    
		    thread::yield_now();
		},
	    }
	}	
    }
}


#[async_trait]
impl ClientHandler for NatsBroker {
    fn get_configuration(&self)->Box<dyn ClientTopicsConfiguration+Send>{
	Box::new(self.nats.clone())
    }
    fn get_subscriptions(&self)->Arc<Mutex<Box<Subscriptions>>>{
	self.subscriptions.clone()
    }
    fn get_type(&self)->String{
	"Nats".to_string()
    }
}

impl NatsBroker {

    pub fn new(config:Nats,rx: Arc<Mutex<mpsc::Receiver<RestToMessagingContext>>>)->Self{
	NatsBroker{
	    nats:config,
	    rx,
	    subscriptions: Arc::new(Mutex::new(Box::new(HashMap::new())))
	}	
    }
      
    async fn nats_event_handler(&self, mut nats: Client) {
        let mut rx = self.rx.lock().await;
        while let Some(job) = rx.next().await {
            let sender = job.sender;
            match job.job {
                Job::Subscribe(value) => {		    
		    self.subscribe(value,sender).await;
                },

		Job::Unsubscribe(value)=>{
		    self.unsubscribe(value,sender).await;
		},
		

                Job::Publish(value) => {
		    let req = value;
                    debug!("Publish {}", req);
                    let maybe_topic = self.nats.get_producer_topic_for_client_topic(&req.client_topic);
                    if let Some(topic) = maybe_topic {
                        let nats_publish = nats.publish(&topic, &req.payload);
                        match nats_publish {
                            Ok(_) => {
                                let res = sender.send(structs::MessagingResult {
				    correlation_id: req.correlation_id,
                                    status: BackendStatusCodes::Ok("NATS is good".to_string()),
                                });
				if res.is_err() {
				    warn!("{:?}",res);
				}
                            }
                            Err(e) => {
                                let res = sender.send(structs::MessagingResult {
				    correlation_id: req.correlation_id,
                                    status: BackendStatusCodes::Error(e.to_string()),
                                });
				if res.is_err(){
				    warn!("{:?}",res);
				}
                            }
                        }
                    } else {
                        warn!("Can't find topic {:?}", req);
                        let res = sender.send(structs::MessagingResult {
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
    
    async fn nats_incoming_event_handler(&self, client: Option<Client>) {        
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
		
		let mut has_lock = false;
		while !has_lock{
		    if let Some(mut subscriptions) = subscriptions.try_lock(){
			if let Some(subscriptions) = subscriptions.get_mut(&topic){			
			    subs = subscriptions.clone();
			}
			has_lock = true;
		    }			
		}
		send_request(&mut subs, event.msg);		
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
	    let mut client = nats::Client::new(cluster).unwrap();
	    client.set_name("swir");
            for ct in self.nats.consumer_topics.iter() {
		
		consumer_topics.push(ct.consumer_topic.clone());
		consumer_groups.push(ct.consumer_group.clone());
		debug!("Subscribing to topic {} {}",ct.consumer_topic,ct.consumer_group);
		client.subscribe(ct.consumer_topic.as_str(), Some(&ct.consumer_group)).unwrap();
            }
	 
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
