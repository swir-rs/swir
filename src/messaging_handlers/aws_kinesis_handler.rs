use std::collections::HashMap;
use std::sync::Arc;
use std::str::FromStr;
use futures::lock::Mutex;
use futures::stream::StreamExt;
use futures::future::join_all;
use bytes;
use tokio::sync::mpsc;
use async_trait::async_trait;
use rand::{Rng, SeedableRng};
use rand::rngs;
use rand::distributions::{Alphanumeric};
use std::time::Duration;

use aws_lock_client::{AwsLockClient,AwsLockClientDynamoDb};

use crate::messaging_handlers::Broker;
use crate::utils::config::AwsKinesis;

use rusoto_kinesis::Kinesis;
use rusoto_core::Region;

use super::super::utils::structs;
use super::super::utils::structs::*;
use super::super::utils::config::ClientTopicsConfiguration;
use crate::messaging_handlers::client_handler::ClientHandler;
use regex::Regex;

lazy_static! {
    static ref RE: Regex = Regex::new(r"0|([1-9]\\d{0,128})").unwrap();
}

type Subscriptions = HashMap<String, Box<Vec<SubscribeRequest>>>;

const DYNAMO_DB_LOCK_TABLE_NAME:&str = "swir-locks";

fn validate_sequence_number(input: &str) -> bool {
    RE.is_match(input)
}

pub struct AwsKinesisBroker {
    aws_kinesis: AwsKinesis,
    rx: Arc<Mutex<mpsc::Receiver<RestToMessagingContext>>>,
    subscriptions: Arc<Mutex<Box<Subscriptions>>>,
}


async fn send_request(subscriptions:  &mut Vec<SubscribeRequest>, p: Vec<u8> ) {
    let msg = String::from_utf8_lossy(&p);
    debug!("Processing message {} {:?}", subscriptions.len(),msg);
    
    for subscription in subscriptions.iter_mut(){

	debug!("Processing subscription {}", subscription);
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
	stream_name:stream_name.clone()
    };
    
    match client.put_record(put_record_input).await{
	Ok(resp)=> {
	    info!("Resp {} {:?}",stream_name, resp);
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
    fn get_subscriptions(&self)->Arc<Mutex<Box<Subscriptions>>>{
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
			let partition_key = rngs::SmallRng::from_entropy().sample_iter(Alphanumeric).take(32).collect();			
			tokio::spawn(async move {
			    debug!("Partition topic {} key {}", topic, partition_key);
			    match aws_put_record(client.clone(),topic,bytes::Bytes::from(req.payload),partition_key).await {
				Ok(())=> {
				    let res = sender.send(structs::MessagingResult {
		     			correlation_id: req.correlation_id,
					status: BackendStatusCodes::Ok("AWS Kinesis is good".to_string())
                                    });
				    if res.is_err() {
					warn!("{:?}",res);
				    }
				},
				Err(())=> {
				    let res = sender.send(structs::MessagingResult {
 					correlation_id: req.correlation_id,
					status: BackendStatusCodes::Error("AWS Kinesis error".to_string()),
				    });
				    if res.is_err() {
					warn!("{:?}",res);
				    }
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



    async fn get_shards_arns(&self,aws_kinesis_client:&rusoto_kinesis::KinesisClient)-> Vec<(String,String, String)>{
	// get arn for all streams on the list
	// register consumers for all arns and get consumer_arn	
	let mut arns =  vec![];
	for ct in self.aws_kinesis.consumer_topics.iter() {
		let consumer_topic = ct.consumer_topic.clone();
		let consumer_group = ct.consumer_group.clone();
		let stream_description_summary_input = rusoto_kinesis::DescribeStreamSummaryInput{
		    stream_name:consumer_topic.clone()
		};
		let stream_description_summary_output = aws_kinesis_client.describe_stream_summary(stream_description_summary_input).await;
		if let Ok(stream_description_summary_output) = stream_description_summary_output{
		    let stream_arn = stream_description_summary_output.stream_description_summary.stream_arn.clone();
		    debug!("Stream {} {} arn {}",consumer_topic,consumer_group, stream_arn);			
		    arns.push((consumer_topic, consumer_group,stream_arn));
		}else{
		    warn!("Error {} {:?}", consumer_topic, stream_description_summary_output.unwrap_err());
		}
	}
	arns
    }

    async fn aws_kinesis_incoming_event_handler(&self, aws_kinesis_client:rusoto_kinesis::KinesisClient) {
	if self.aws_kinesis.consumer_topics.is_empty(){
	    info!("No consumers configured, bye");
	    return
	}
	
	let aws_lock_client = AwsLockClientDynamoDb::new(Region::EuWest1,DYNAMO_DB_LOCK_TABLE_NAME.to_string());
	
	loop{
	    let arns = self.get_shards_arns(&aws_kinesis_client).await;    	    
	    let now = tokio::time::Instant::now();
	    let interval_duration = tokio::time::Duration::from_millis(5000);
	    let mut tasks = vec![];
	    
	    for desc in arns.iter() {
		let desc = desc.clone();
		let aws_kinesis_client = aws_kinesis_client.clone();
		let aws_lock_client = aws_lock_client.clone();
		// starting a task per stream name
		let subscriptions = self.subscriptions.clone();
		let handle = tokio::spawn(async move {
		    let subscriptions = subscriptions;
		    let (stream_name, group_name,_) = desc.clone();
	    	    let list_shards_input = rusoto_kinesis::ListShardsInput{
	    		exclusive_start_shard_id: None,
	    		max_results: None,
	    		next_token: None,
	    		stream_creation_timestamp: None,
	    		stream_name: Some(stream_name.clone())
	    	    };		   
	    	    let list_shards_output = aws_kinesis_client.list_shards(list_shards_input).await;		    		   		    
	    	    if let Ok(list_shards_output) = list_shards_output{
	    		if let Some(shards) = list_shards_output.shards{
			    // iterating over all shards, only processing data from a shard when lock succeeds 
	    		    for shard in shards.iter(){
	    			debug!("Stream {:?} shard id {} shard {:?}",desc, shard.shard_id,shard);
				let aws_lock_client = aws_lock_client.clone();
				let shard_id = shard.shard_id.clone();
				let key = format!("{}-{}-{}",stream_name.clone(), group_name,shard_id);
				let mut shard_sequence_number = shard.sequence_number_range.starting_sequence_number.clone();
				let maybe_lock = aws_lock_client.try_acquire_lock(key.clone(),Duration::from_millis(10000)).await;
				debug!("Lock acquired {:?}", maybe_lock);
				
				let mut lock = if let Ok(lock) = maybe_lock{
				    lock
				}else{
				    return;
				};
				let maybe_sequence_number = lock.lock_data.clone().unwrap_or_else(|| "".to_string());
				
				if validate_sequence_number(&maybe_sequence_number){				    
				    shard_sequence_number = maybe_sequence_number;
				}
								
				let get_shard_iterator_input =  rusoto_kinesis::GetShardIteratorInput{
				    shard_id:shard_id.clone(),
				    shard_iterator_type:String::from("AFTER_SEQUENCE_NUMBER"),
				    starting_sequence_number:Some(shard_sequence_number.clone()),
				    stream_name:stream_name.clone(),
				    timestamp:None
				};
				
				let get_shard_iterator_output = aws_kinesis_client.get_shard_iterator(get_shard_iterator_input).await;


				match get_shard_iterator_output {				    
				    Ok(get_shard_iterator_output) => {				
					debug!("Shard iterator {:?}", get_shard_iterator_output);
					let mut maybe_shard_iterator = get_shard_iterator_output.shard_iterator;
					
					while let Some(shard_iterator) = maybe_shard_iterator{
					    let get_records_input = rusoto_kinesis::GetRecordsInput{
						limit: Some(500),
						shard_iterator
					    };
					    
					    match aws_kinesis_client.get_records(get_records_input).await{		
						Ok(get_records_output) => {
						    info!("Records {} {} {}",stream_name, shard_id.clone(),get_records_output.records.len());
						    for record in get_records_output.records.iter(){
							debug!("Record {} {} {:?}", stream_name, shard_id.clone(),record);							
							let vec = record.data.to_vec();		    		    
							let mut subscriptions = subscriptions.lock().await;		    
							if let Some(mut subs) = subscriptions.get_mut(&stream_name){
							    if subs.len()!=0{
								send_request(&mut subs, vec).await;
							    }else{
								warn!("No subscriptions for {} {}",stream_name,String::from_utf8_lossy(&vec));
							    }
							}							
							shard_sequence_number = record.sequence_number.clone();
							
						    }
						    if !get_records_output.records.is_empty() {
							maybe_shard_iterator = get_records_output.next_shard_iterator;
						    }else{
							maybe_shard_iterator = None;
						    }
						},
						Err(e)=>{
						    warn!("Error {:?}",e);
						    maybe_shard_iterator= None;
						}
					    }
 	    				}
				    },
				    Err(e)=>{
					warn!("Error {:?}",e);
				    }
				}
				lock.lock_data = Some(shard_sequence_number);
				let r = aws_lock_client.release_lock(key, lock).await;
				debug!("Lock released {:?}",r);
	    		    }
			}			
	    	    }else{
			warn!("Error {:?}", list_shards_output.unwrap_err());
		    }
		    
		});	    
		tasks.push(handle);
	    }
	    join_all(tasks).await;
	    let elapsed = now.elapsed();
	    if elapsed < interval_duration{
		let additional_delay = interval_duration-elapsed;
		debug!("Additional delay of {:?}", additional_delay);
		tokio::time::delay_for(interval_duration-elapsed).await;		
	    }				
	}
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
