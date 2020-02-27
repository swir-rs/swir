use std::fmt;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use futures::{StreamExt};
use futures::channel::oneshot;
use futures::lock::Mutex;
use hyper::StatusCode;
use tokio::sync::mpsc;
use tonic::{Response, Status};
use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;
use base64;

use crate::utils::structs::CustomerInterfaceType::GRPC;
use crate::utils::structs::{EndpointDesc, Job, MessagingResult, MessagingToRestContext, RestToMessagingContext};

pub mod client_api {
    tonic::include_proto!("swir");
}

impl fmt::Display for client_api::SubscribeRequest{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SubscribeRequest {{ correlation_id:{}, topic:{}}}", &self.correlation_id,&self.topic)
    }
}

impl fmt::Display for client_api::SubscribeResponse{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SubscribeResponse {{ correlation_id:{}, payload:{}}}", &self.correlation_id,String::from_utf8_lossy(&self.payload))
    }
}

impl fmt::Display for client_api::PublishRequest{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PublishRequest {{ correlation_id:{}, topic: {}, payload:{} }}", &self.correlation_id, &self.topic, String::from_utf8_lossy(&self.payload))
    }
}

impl fmt::Display for client_api::PublishResponse{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PublishResponse {{ correlation_id:{}, status: {} }}",&self.correlation_id,  &self.status)
    }
}

#[derive(Debug)]
pub struct SwirAPI {
    missed_messages: Arc<Mutex<Box<VecDeque<client_api::SubscribeResponse>>>>,
    pub from_client_to_backend_channel_sender: Box<HashMap<String, Box<mpsc::Sender<RestToMessagingContext>>>>,
    pub to_client_receiver: Arc<Mutex<mpsc::Receiver<crate::utils::structs::MessagingToRestContext>>>,
}

impl SwirAPI {
    fn find_channel(&self, topic_name: &String) -> Option<&Box<mpsc::Sender<RestToMessagingContext>>> {
        self.from_client_to_backend_channel_sender.get(topic_name)
    }

    pub fn new(
        from_client_to_backend_channel_sender: Box<HashMap<String, Box<mpsc::Sender<RestToMessagingContext>>>>,
        to_client_receiver: Arc<Mutex<mpsc::Receiver<MessagingToRestContext>>>,
    ) -> SwirAPI {
        let missed_messages = Arc::new(Mutex::new(Box::new(VecDeque::new())));
        SwirAPI {
            missed_messages,
            from_client_to_backend_channel_sender,
            to_client_receiver,
        }
    }
}

#[tonic::async_trait]
impl client_api::client_api_server::ClientApi for SwirAPI {    

    type PublishBiStreamStream =
	mpsc::Receiver<Result<client_api::PublishResponse, Status>>;
    
    
    async fn publish_bi_stream(&self, request: tonic::Request<tonic::Streaming<client_api::PublishRequest>>) -> Result<tonic::Response<Self::PublishBiStreamStream>, tonic::Status> {
	info!("Publish bidi stream");
        let stream = request.into_inner();
        let (mut tx, rx) = mpsc::channel(10);
	let (mut internal_tx, mut internal_rx): (mpsc::Sender<client_api::PublishResponse>, mpsc::Receiver<client_api::PublishResponse>) = mpsc::channel(1);

	let error1 = Arc::new(AtomicBool::new(false));
	let error2 = error1.clone();

	let channels = self.from_client_to_backend_channel_sender.clone();
        tokio::spawn(
	    async move {
		let mut cond = false;
		let error = error1.clone();
		while !cond{
		    let response = internal_rx.next().await;		
		    match response{		
			Some(response)=>{
			    debug!("Got message {}",response);
			    let pr :client_api::PublishResponse = response.clone();
                            let r = tx.send(Ok(pr.clone())).await; //.expect("I should not panic as I should not be here!");
			    if let Err(_) = r {			
				error.swap(true,Ordering::Relaxed);
				info!("gRPC connection closed for message {}",pr);
				break;
			    }else{
				debug!("Message sent {}",pr);
			    }
			},
			None=>{
			    info!("Internal channel closed");
			    error.swap(true,Ordering::Relaxed);
			    break;
			}
		    }
		    cond= error.load(Ordering::Relaxed);
		}
		info!("publish_bi_strean sender 1 terminated");
            }            
        );

	tokio::spawn(
	    async move{
		futures::pin_mut!(stream);
		let mut cond = false;
		let error = error2.clone();
		while !cond{
		    let error = error.clone();
		    let request = stream.next().await;
		    match request{
			Some(request)=> {
			    if let Ok(request) = request{
				info!("Publish request {}", request);
				let mut msg = String::new();
				
				if let Some(tx) = channels.get(&request.topic) {
				    let p = crate::utils::structs::PublishRequest {				    
					correlation_id: request.correlation_id.clone(),
					payload: request.payload,
					client_topic: request.topic.clone(),
				    };
				    
				    let (local_tx, local_rx): (oneshot::Sender<MessagingResult>, oneshot::Receiver<MessagingResult>) = oneshot::channel();
				    let job = RestToMessagingContext {
					job: Job::Publish(p),
					sender: local_tx,
				    };
				    
				    let mut tx = tx.clone();
				    if let Err(e) = tx.try_send(job) {
					warn!("Channel is dead {:?}", e);
				    }
				    
				    let response_from_broker: Result<MessagingResult, oneshot::Canceled> = local_rx.await;
				    if let Ok(res) = response_from_broker {		    
					msg.push_str(&res.status.to_string());
				    } else {
					msg.push_str("problem with backend");
				    }		
				} else {
				    msg.push_str("Invalid token");
				}
				let reply = client_api::PublishResponse {
				    correlation_id: request.correlation_id,
				    status: format!("{}", msg).into(), // We must use .into_inner() as the fields of gRPC requests and responses are private
				};
				debug!("Sending internally  {}",reply);
				let r = internal_tx.send(reply.clone()).await;
				
				if let Err(_) =r{
				    info!("Internal channel closed for message {}", reply);
				    error.swap(true,Ordering::Relaxed);
				    break;
				}
			    }
			},
			None=>{
			    debug!("End of stream");
			    error.swap(true,Ordering::Relaxed);			    
			}
		    }
		    cond= error.load(Ordering::Relaxed);		    
		}
		info!("publish_bi_stream sender 2 terminated");
	    }
	);
	Ok(tonic::Response::new(rx))	    
    }

    async fn publish(
        &self,
        request: tonic::Request<client_api::PublishRequest>, // Accept request of type HelloRequest
    ) -> Result<tonic::Response<client_api::PublishResponse>, tonic::Status> {
        let request = request.into_inner();
        info!("Publish {}", request);
        if let Some(tx) = self.find_channel(&request.topic) {
            let p = crate::utils::structs::PublishRequest {
		correlation_id: request.correlation_id.clone(),
                payload: request.payload,
                client_topic: request.topic.clone(),
            };
            debug!("{}", p);
            let (local_tx, local_rx): (oneshot::Sender<MessagingResult>, oneshot::Receiver<MessagingResult>) = oneshot::channel();
            let job = RestToMessagingContext {
                job: Job::Publish(p),
                sender: local_tx,
            };

            let mut tx = tx.clone();
            if let Err(e) = tx.try_send(job) {
                warn!("Channel is dead {:?}", e);
            }
	   
            let response_from_broker: Result<MessagingResult, oneshot::Canceled> = local_rx.await;	   
            let mut msg = String::new();
	    
            if let Ok(res) = response_from_broker {
                msg.push_str(&res.status.to_string());
            } else {
                msg.push_str(StatusCode::INTERNAL_SERVER_ERROR.as_str());
            }
            let reply = client_api::PublishResponse {
		correlation_id: request.correlation_id,
                status: format!("{}", msg).into(), // We must use .into_inner() as the fields of gRPC requests and responses are private
            };
            Ok(tonic::Response::new(reply)) // Send back our formatted greeting
        } else {
            Err(tonic::Status::invalid_argument("Invalid topic"))
        }
    }

    type SubscribeStream = mpsc::Receiver<Result<client_api::SubscribeResponse, Status>>;

    async fn subscribe(&self, request: tonic::Request<client_api::SubscribeRequest>) -> Result<tonic::Response<Self::SubscribeStream>, tonic::Status> {
	let request = request.into_inner();
	info!("Subscribe {}", request);
        let topic = request.topic;
	let correlation_id = request.correlation_id;	
	let (to_client_tx,mut to_client_rx) = mpsc::channel::<MessagingToRestContext>(1000);
	let mut small_rng = SmallRng::from_entropy();
	let mut array: [u8; 32]=[0;32];
	small_rng.fill(&mut array);
	let client_id = base64::encode(&array);	
        let sr = crate::utils::structs::SubscribeRequest {
	    correlation_id:correlation_id.clone(),
            endpoint: EndpointDesc { url: "".to_string(), client_id:client_id },
            client_topic: topic.clone(),
            client_interface_type: GRPC,
	    tx: Box::new(to_client_tx)
        };
	
        let (local_tx, _local_rx): (oneshot::Sender<MessagingResult>, oneshot::Receiver<MessagingResult>) = oneshot::channel();
        let subscribe_job = RestToMessagingContext {
            job: Job::Subscribe(sr.clone()),
            sender: local_tx,
        };

	let mut txx;
        if let Some(tx) = self.find_channel(&topic) {
            txx = tx.clone();
            if let Err(e) = txx.try_send(subscribe_job) {
                warn!("Channel is dead {:?}", e);
		return Err(tonic::Status::internal("Internal error"));
            }
        } else {
            return Err(tonic::Status::invalid_argument("Invalid topic"));
        }

        let (mut tx, rx) = mpsc::channel(1000);
        tokio::spawn(async move {
	    let mut msgs:i32  = 0;
	    while let Some(messaging_context) = to_client_rx.recv().await{
		let s = client_api::SubscribeResponse { correlation_id:correlation_id.clone(), payload: messaging_context.payload };
		msgs+=1;
		debug!("Sending message {}",s);
		let r = tx.send(Ok(s.clone())).await; //.expect("I should not panic as I should not be here!");
		if let Err(_) = r {
		    info!("Message pushed back {}", s);
		    let (unsub_tx, _unsub_rx): (oneshot::Sender<MessagingResult>, oneshot::Receiver<MessagingResult>) = oneshot::channel();	
		    let unsubscribe_job = RestToMessagingContext {
			job: Job::Unsubscribe(sr.clone()),
			sender: unsub_tx,
		    };
		    
		    if let Err(e) = txx.try_send(unsubscribe_job) {
			warn!("Channel is dead {:?}", e);
		    }
		    debug!("Messages processed in this session {}",msgs);
		}
	    }
	});

        Ok(Response::new(rx))
    }
}
