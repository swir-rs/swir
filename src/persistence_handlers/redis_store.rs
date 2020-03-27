
use crate::utils::structs::DeleteRequest;
use async_trait::async_trait;
use crate::utils::config::Redis;
use tokio::stream::StreamExt;
use crate::utils::structs::{RestToPersistenceContext,PersistenceJobType,StoreRequest,RetrieveRequest,PersistenceResult,BackendStatusCodes};
use crate::persistence_handlers::Store;
use tokio::sync::mpsc;
use futures::channel::oneshot::Sender;
use std::sync::Arc;
use futures::lock::Mutex;


use redis::{pipe, Commands, Client, Connection};


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
	let res: Result<(Option<String>,String),redis::RedisError> = pipe().atomic().get(&sr.key).set(&sr.key, sr.payload).query(connection);
	info!("{:?}",res);
	let pr = match res{
	    Ok((r1,r2)) => {
		let payload = if let Some(data) = r1 {
		    data.into_bytes()
		}else{
		    vec![]
		};
		let status = if r2 == "OK" {
		    BackendStatusCodes::Ok("REDIS is good".to_string())
		}else{
		    BackendStatusCodes::Ok(format!("Problem when storing key: {:?}",r2).to_string())
		};
			    
		PersistenceResult{
		    correlation_id: sr.correlation_id,
		    status,
		    payload,
		}
	    },
	    
	    Err(e) => {
		PersistenceResult{
		    correlation_id: sr.correlation_id,
		    status: BackendStatusCodes::Error(e.to_string()),
		    payload: vec![]
		}
	    }
	};
	let r = sender.send(pr);
	if r.is_err() {
	    warn!("Can't send response {:?}",r);
	};
    }

    fn retrieve(&self, connection:&mut Connection,sr:RetrieveRequest,sender:Sender<PersistenceResult>){
	let r:Result<String,redis::RedisError> = connection.get(sr.key);
	let rr = match r {
	    Ok(data) => {
		PersistenceResult{
		    correlation_id: sr.correlation_id,
		    status: BackendStatusCodes::Ok("REDIS is good".to_string()),
		    payload: data.into_bytes()
		}
	    },
	    
	    Err(e) => {
		PersistenceResult{
		    correlation_id: sr.correlation_id,
		    status: BackendStatusCodes::Error(e.to_string()),
		    payload: vec![]
		}		
	    }	    
	};
	
	let r = sender.send(rr);
	if r.is_err() {
	    warn!("Can't send response {:?}",r);
	};
    }

    fn delete(&self, connection:&mut Connection,sr:DeleteRequest,sender:Sender<PersistenceResult>){
	let res: Result<(Option<String>,i32),redis::RedisError> = pipe().atomic().get(&sr.key).del(&sr.key).query(connection);
	info!("{:?}",res);
	let dr = match res{
	    Ok((r1,r2)) => {
		let payload = if let Some(data) = r1 {
		    data.into_bytes()
		}else{
		    vec![]
		};
		let status = BackendStatusCodes::Ok(format!("Deleted keys : {:?}",r2).to_string());
			    
		PersistenceResult{
		    correlation_id: sr.correlation_id,
		    status,
		    payload,
		}
	    },
	    
	    Err(e) => {
		PersistenceResult{
		    correlation_id: sr.correlation_id,
		    status: BackendStatusCodes::Error(e.to_string()),
		    payload: vec![]
		}
	    }
	};
			
	let r = sender.send(dr);
	if r.is_err() {
	    warn!("Can't send response {:?}",r);
	};
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
		PersistenceJobType::Delete(value)=>{
		    self.delete(&mut connection,value,sender);

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


