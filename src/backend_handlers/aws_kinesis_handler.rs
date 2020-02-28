use std::collections::HashMap;
use std::sync::Arc;
use std::str::FromStr;
use futures::lock::Mutex;
use futures::stream::StreamExt;
use bytes;
use tokio::sync::mpsc;
use async_trait::async_trait;
use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;
use rand::distributions::{Alphanumeric};
use base64;

use crate::backend_handlers::Broker;
use crate::utils::config::AwsKinesis;

use rusoto_kinesis::Kinesis;

use super::super::utils::structs;
use super::super::utils::structs::*;
use super::super::utils::config::ClientTopicsConfiguration;
use crate::backend_handlers::client_handler::ClientHandler;




pub struct AwsKinesisBroker {
    aws_kinesis: AwsKinesis,
    rx: Arc<Mutex<mpsc::Receiver<RestToMessagingContext>>>,
    subscriptions: Arc<Mutex<Box<HashMap<String, Box<Vec<SubscribeRequest>>>>>>,
}


async fn send_request(subscriptions:  &mut Box<Vec<SubscribeRequest>>, p: Vec<u8> ) {
    let msg = String::from_utf8_lossy(&p);
    debug!("Processing message {} {:?}", subscriptions.len(),msg);
    
    for subscription in subscriptions.iter_mut(){
	let (s, _r) = futures::channel::oneshot::channel();
	debug!("Processing subscription {}", subscription);
	let mrc = MessagingToRestContext {
	    sender: s,
	    payload: p.to_vec(),
	    uri: subscription.endpoint.url.clone(),
        };
	match subscription.tx.send(mrc).await{
	    Ok(_) => {
		debug!("Message sent {:?}",msg);
	    },
	    
	    Err(mpsc::error::SendError(_)) => {
		warn!("Unable to send {}. Channel is closed", subscription);
	    },
	}
    }
}

async fn aws_put_record(client:rusoto_kinesis::KinesisClient, stream_name:String, payload: bytes::Bytes, partition_key: String)->Result<(),()>{
    let put_record_input = rusoto_kinesis::PutRecordInput{
		data: payload,
		explicit_hash_key: None,
		partition_key,
		sequence_number_for_ordering: None,
		stream_name
	    };
    
    match client.put_record(put_record_input).await{
		Ok(resp)=> {
		    info!("Resp {:?}",resp);
		    Ok(())
		},
		Err(e)=>{error!("Err {:?}",e); Err(()) }
	    }
}


#[async_trait]
impl ClientHandler for AwsKinesisBroker {
    fn get_configuration(&self)->Box<dyn ClientTopicsConfiguration+Send>{
	Box::new(self.aws_kinesis.clone())
    }
    fn get_subscriptions(&self)->Arc<Mutex<Box<HashMap<String, Box<Vec<SubscribeRequest>>>>>>{
	self.subscriptions.clone()
    }
    fn get_type(&self)->String{
	"AwsKinesis".to_string()
    }
}

impl AwsKinesisBroker {
    pub fn new(config:AwsKinesis,rx: Arc<Mutex<mpsc::Receiver<RestToMessagingContext>>>)->Self{
	
	AwsKinesisBroker{
	    aws_kinesis:config,
	    rx,
	    subscriptions: Arc::new(Mutex::new(Box::new(HashMap::new())))
	}	
    }
    
    async fn aws_kinesis_event_handler(&self,aws_kinesis_client:rusoto_kinesis::KinesisClient) {
	
        info!("Aws Kinesis running {:?}",self.aws_kinesis );
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
		    let maybe_topic = self.aws_kinesis.get_producer_topic_for_client_topic(&req.client_topic);
		    let client = aws_kinesis_client.clone();
                    if let Some(topic) = maybe_topic {
			let partition_key = SmallRng::from_entropy().sample_iter(Alphanumeric).take(32).collect();			
			tokio::spawn(async move {
			    debug!("Partition topic {} key {}", topic, partition_key);
			    match aws_put_record(client.clone(),topic,bytes::Bytes::from(req.payload),partition_key).await {
				Ok(())=> {
				    sender.send(structs::MessagingResult {
		     			correlation_id: req.correlation_id,
					status: BackendStatusCodes::Ok("AWS Kinesis is good".to_string())
                                    });
				},
				Err(())=> {
				    sender.send(structs::MessagingResult {
 					correlation_id: req.correlation_id,
					status: BackendStatusCodes::Error("AWS Kinesis error".to_string()),
				    });
				}
			    }
			});
                    } else {
			warn!("Can't find topic {}", req);
			if let Err(e) = sender.send(structs::MessagingResult {
		     	    correlation_id: req.correlation_id,
                            status: BackendStatusCodes::NoTopic("Can't find subscribe topic".to_string()),
			}) {
                            warn!("Can't send response back {:?}", e);
			}
                    }
		}
            }
	}
    }



    async fn aws_kinesis_incoming_event_handler(&self, aws_kinesis_client:rusoto_kinesis::KinesisClient) {


        let mut consumer_topics = vec![];
        let mut consumer_groups = vec![];

	if self.aws_kinesis.consumer_topics.is_empty(){
	    info!("No consumers configured, bye");
	    return
	}
        for ct in self.aws_kinesis.consumer_topics.iter() {
            consumer_topics.push(ct.consumer_topic.clone());
            consumer_groups.push(ct.consumer_group.clone());
        }

        let consumer_group = consumer_groups.get(0).unwrap();


    }
}

#[async_trait]
impl Broker for AwsKinesisBroker {
    async fn configure_broker(&self) {
        info!("Configuring Aws Kinesis broker {:?} ", self.aws_kinesis);	
	if self.aws_kinesis.regions.len() != 1{
	    warn!("Invalid regions {:?}",self.aws_kinesis.regions);
	    return;
	}
	
	let region = if let Ok(region) = rusoto_signature::Region::from_str(&self.aws_kinesis.regions[0]){
	    region
	}else{
	    warn!("Unknown region {}",self.aws_kinesis.regions[0]);
	    return;
	};

	let kinesis_client = rusoto_kinesis::KinesisClient::new(region);
			
        let f1 = async { self.aws_kinesis_incoming_event_handler(kinesis_client.clone()).await };
        let f2 = async { self.aws_kinesis_event_handler(kinesis_client.clone()).await };
        let (_r1, _r2) = futures::join!(f1, f2);
    }
}
