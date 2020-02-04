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

use crate::utils::structs::CustomerInterfaceType::GRPC;
use crate::utils::structs::{EndpointDesc, Job, MessagingResult, MessagingToRestContext, RestToMessagingContext};

pub mod client_api {
    tonic::include_proto!("swir");
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
	info!("Publish bi stream called");
        let stream = request.into_inner();

        let (mut tx, rx) = mpsc::channel(10);
	let (mut internal_tx, mut internal_rx): (mpsc::Sender<client_api::PublishResponse>, mpsc::Receiver<client_api::PublishResponse>) = mpsc::channel(10);



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
			    debug!("Got message {:?}",response);
			    let pr :client_api::PublishResponse = response.clone();
                            let r = tx.send(Ok(pr.clone())).await; //.expect("I should not panic as I should not be here!");

			    if let Err(e) = r {			
				info!("Message not sent {:?}", e);
				info!("Message pushed back {:?}", pr);
				error.swap(true,Ordering::Relaxed);
				warn!("Bailing out in channel loop. Can't send out");
				break;
			    }
			},
			None=>{
			    warn!("Bailing out in channel loop. Received nothing on internal");
			    error.swap(true,Ordering::Relaxed);
			    break;
			}
		    }
		    cond= error.load(Ordering::Relaxed);
		}
		debug!("publish_bi_strean sender 1 terminated");
            }            
        );

	tokio::spawn(
	    async move{
		futures::pin_mut!(stream);
		info!("Pinned");
		let mut cond = false;
		let error = error2.clone();
		while !cond{
		    let error = error.clone();
		    let request = stream.next().await;
		    match request{
			Some(request)=> {
			    if let Ok(request) = request{
				debug!("  ==> publish request  {:?}", request);
				let mut msg = String::new();
				
				if let Some(tx) = channels.get(&request.topic) {
				    let p = crate::utils::structs::PublishRequest {				    
					payload: request.payload,
					client_topic: request.topic.clone(),
				    };
				debug!("{:?}", p);
				    let (local_tx, local_rx): (oneshot::Sender<MessagingResult>, oneshot::Receiver<MessagingResult>) = oneshot::channel();
				    let job = RestToMessagingContext {
					job: Job::Publish(p),
					sender: local_tx,
				    };
				    
				    let mut tx = tx.clone();
				    if let Err(e) = tx.try_send(job) {
					warn!("Channel is dead {:?}", e);
				    }
				    
				    debug!("Waiting for broker");
				    let response_from_broker: Result<MessagingResult, oneshot::Canceled> = local_rx.await;
				    debug!("Got result {:?}", response_from_broker);
				    if let Ok(res) = response_from_broker {		    
					msg.push_str(&res.status.to_string());
				    } else {
					msg.push_str("problem with backend");
				    }		
				} else {
				    msg.push_str("Invalid token");
				}
				let reply = client_api::PublishResponse {
				    status: format!("{}", msg).into(), // We must use .into_inner() as the fields of gRPC requests and responses are private
			    };
			    debug!("Sending internally  {:?}",reply);
				let r = internal_tx.send(reply).await;
				
				if let Err(e) =r{
				    info!("Message not sent {:?}", e);
				    error.swap(true,Ordering::Relaxed);
				    warn!("Bailing out in channel loop. Can't send out");
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
		debug!("publish_bi_strean sender 2 terminated");
	    }
	);
	Ok(tonic::Response::new(rx))	    
    }

    async fn publish(
        &self,
        request: tonic::Request<client_api::PublishRequest>, // Accept request of type HelloRequest
    ) -> Result<tonic::Response<client_api::PublishResponse>, tonic::Status> {
        // Return an instance of type HelloReply
        debug!("Got a request: {:?}", request);
        let request = request.into_inner();

        if let Some(tx) = self.find_channel(&request.topic) {
            let p = crate::utils::structs::PublishRequest {
                payload: request.payload,
                client_topic: request.topic.clone(),
            };
            debug!("{:?}", p);
            let (local_tx, local_rx): (oneshot::Sender<MessagingResult>, oneshot::Receiver<MessagingResult>) = oneshot::channel();
            let job = RestToMessagingContext {
                job: Job::Publish(p),
                sender: local_tx,
            };

            let mut tx = tx.clone();
            if let Err(e) = tx.try_send(job) {
                warn!("Channel is dead {:?}", e);
            }

            debug!("Waiting for broker");
            let response_from_broker: Result<MessagingResult, oneshot::Canceled> = local_rx.await;
            debug!("Got result {:?}", response_from_broker);
            let mut msg = String::new();
            if let Ok(res) = response_from_broker {
                msg.push_str(&res.status.to_string());
            } else {
                msg.push_str(StatusCode::INTERNAL_SERVER_ERROR.as_str());
            }
            let reply = client_api::PublishResponse {
                status: format!("{}", msg).into(), // We must use .into_inner() as the fields of gRPC requests and responses are private
            };
            Ok(tonic::Response::new(reply)) // Send back our formatted greeting
        } else {
            Err(tonic::Status::invalid_argument("Invalid topic"))
        }
    }

    type SubscribeStream = mpsc::Receiver<Result<client_api::SubscribeResponse, Status>>;

    async fn subscribe(&self, request: tonic::Request<client_api::SubscribeRequest>) -> Result<tonic::Response<Self::SubscribeStream>, tonic::Status> {
        let topic = request.into_inner().topic;
        info!("Topic = {:?}", topic);


	let (to_client_tx,mut to_client_rx) = mpsc::channel::<MessagingToRestContext>(1000);
	
        let sr = crate::utils::structs::SubscribeRequest {
            endpoint: EndpointDesc { url: "".to_string() },
            client_topic: topic.clone(),
            client_interface_type: GRPC,
	    tx: Box::new(to_client_tx)
        };

        let (local_tx, _local_rx): (oneshot::Sender<MessagingResult>, oneshot::Receiver<MessagingResult>) = oneshot::channel();
        let job = RestToMessagingContext {
            job: Job::Subscribe(sr),
            sender: local_tx,
        };
        info!("About to send to messaging processor");

        if let Some(tx) = self.find_channel(&topic) {
            let mut tx = tx.clone();
            if let Err(e) = tx.try_send(job) {
                warn!("Channel is dead {:?}", e);
            }
        } else {
            return Err(tonic::Status::invalid_argument("Invalid topic"));
        }

        let (mut tx, rx) = mpsc::channel(10);
        let missed_messages = self.missed_messages.clone();
//        let loc_rx = self.to_client_receiver.clone();

        tokio::spawn(async move {
            let mut missed_messages = missed_messages.lock().await;
            while let Some(s) = missed_messages.pop_front() {
                let r = tx.send(Ok(s.clone())).await; //.expect("I should not panic as I should not be here!");
                info!("Message sent from the queue {:?} {:?}", r, s);
                if let Err(_) = r {
                    info!("Message pushed front  {:?}", s);
                    missed_messages.push_front(s);
                    return;
                }
            }

	    while let Some(messaging_context) = to_client_rx.next().await {
                let s = client_api::SubscribeResponse { payload: messaging_context.payload };
                let r = tx.send(Ok(s.clone())).await; //.expect("I should not panic as I should not be here!");
                if let Err(_) = r {
                    info!("Message pushed back  {:?}", s);
                    missed_messages.push_back(s);
                    return;
                }
            }
	    
   

            // let loc_rx = loc_rx.try_lock();
            // if loc_rx.is_some() {
            //     if let Some(mut lrx) = loc_rx {
            //         info!("Lock acquired {:?}", lrx);
            //         while let Some(messaging_context) = lrx.next().await {
            //             let s = client_api::SubscribeResponse { payload: messaging_context.payload };
            //             let r = tx.send(Ok(s.clone())).await; //.expect("I should not panic as I should not be here!");
            //             if let Err(_) = r {
            //                 info!("Message pushed back  {:?}", s);
            //                 missed_messages.push_back(s);
            //                 return;
            //             }
            //         }
            //     }
            // } else {
            //     warn!("Unable to lock the rx");
            // };
        });

        Ok(Response::new(rx))
    }
}
