
use async_trait::async_trait;
use crate::utils::config::Redis;
use tokio::stream::StreamExt;
use crate::utils::structs::{RestToPersistenceContext,PersistenceJobType,StoreRequest,RetrieveRequest,PersistenceResult,BackendStatusCodes};
use crate::persistence_handlers::Store;
use tokio::sync::mpsc;
use futures::channel::oneshot::Sender;
use std::sync::Arc;
use futures::lock::Mutex;

use redis::Commands;
use redis::Client;
use redis::Connection;


#[derive(Debug)]
pub struct RedisStore<'l>{
    config:&'l Redis,
    rx: Arc<Mutex<mpsc::Receiver<RestToPersistenceContext>>>,    
}

impl<'l> RedisStore<'l>{    
    pub fn new(config:&'l Redis,rx: Arc<Mutex<mpsc::Receiver<RestToPersistenceContext>>>)->Self{	
	RedisStore {
	    config,
	    rx,		
	}	
    }
    fn store(&self, connection:&mut Connection, sr:StoreRequest,sender:Sender<PersistenceResult>){
	let r:Result<(),redis::RedisError> = connection.set(sr.key,sr.payload);
	match r{
	    Ok(()) => {
		let pr = PersistenceResult{
		    correlation_id: sr.correlation_id,
		    status: BackendStatusCodes::Ok("REDIS is good".to_string()),
		    payload: vec![]
		};
		let r = sender.send(pr);
		if r.is_err() {
		    warn!("Can't send response {:?}",r);
		};
	    },
	    
	    Err(e) => {
		let pr = PersistenceResult{
		    correlation_id: sr.correlation_id,
		    status: BackendStatusCodes::Error(e.to_string()),
		    payload: vec![]
		};
		let r = sender.send(pr);
		if r.is_err() {
		    warn!("Can't send response {:?}",r);
		};		
	    }
	}
    }

    fn retrieve(&self, connection:&mut Connection,sr:RetrieveRequest,sender:Sender<PersistenceResult>){
	let r:Result<String,redis::RedisError> = connection.get(sr.key);
	match r{
	    Ok(data) => {
		let pr = PersistenceResult{
		    correlation_id: sr.correlation_id,
		    status: BackendStatusCodes::Ok("REDIS is good".to_string()),
		    payload: data.into_bytes()
		};
		let r = sender.send(pr);
		if r.is_err() {
		    warn!("Can't send response {:?}",r);
		};
	    },
	    
	    Err(e) => {
		let pr = PersistenceResult{
		    correlation_id: sr.correlation_id,
		    status: BackendStatusCodes::Error(e.to_string()),
		    payload: vec![]
		};
		let r = sender.send(pr);
		if r.is_err() {
		    warn!("Can't send response {:?}",r);
		};		
	    }	    
	}
    }
    

    async fn event_handler(&self) {
	let client = Client::open(self.config.nodes[0].clone()).unwrap();
	let mut connection = client.get_connection().unwrap();
	
	info!("Redis is running");
	let mut rx = self.rx.lock().await;	
        while let Some(job) = rx.next().await {
            let sender = job.sender;
            match job.job {
                PersistenceJobType::Store(value) => {
		    self.store(&mut connection,value,sender);
                },

		PersistenceJobType::Retrieve(value)=>{
		    self.retrieve(&mut connection,value,sender);
		},
	    }
	}
    }
}

#[async_trait]
impl<'l> Store for RedisStore<'l> {
    async fn configure_store(&self){
	info!("Configuring Redis store {:?} ", self);
        let f1 = async { self.event_handler().await };
        let _res = f1.await;
	
    }
}


