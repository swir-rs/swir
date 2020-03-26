use async_trait::async_trait;
use tokio::sync::mpsc;
use crate::utils::config::DynamoDb;
use crate::utils::structs::{RestToPersistenceContext,PersistenceJobType,StoreRequest,RetrieveRequest,PersistenceResult,BackendStatusCodes};
use crate::persistence_handlers::Store;
use tokio::stream::StreamExt;
use futures::channel::oneshot::Sender;
use std::sync::Arc;
use futures::lock::Mutex;

#[derive(Debug)]
pub struct DynamoDbStore<'l>{
    config:&'l DynamoDb,
    rx: Arc<Mutex<mpsc::Receiver<RestToPersistenceContext>>>
}


impl<'l> DynamoDbStore<'l>{    
    pub fn new(config:&'l DynamoDb,rx: Arc<Mutex<mpsc::Receiver<RestToPersistenceContext>>>)->Self{
	DynamoDbStore{
	    config,
	    rx
	}	
    }
    fn store(&self, sr:StoreRequest,sender:Sender<PersistenceResult>){
	let pr = PersistenceResult{
	    correlation_id: sr.correlation_id,
	    status: BackendStatusCodes::Error("DynamoDB is not implemented".to_string()),
	    payload: vec![]
	};
	let r = sender.send(pr);
	if r.is_err() {
	    warn!("Can't send response {:?}",r);
	};		
    }

    fn retrieve(&self, sr:RetrieveRequest,sender:Sender<PersistenceResult>){
	let pr = PersistenceResult{
	    correlation_id: sr.correlation_id,
	    status: BackendStatusCodes::Error("DynamoDB is not implemented".to_string()),
	    payload: vec![]
	};
	let r = sender.send(pr);
	if r.is_err() {
	    warn!("Can't send response {:?}",r);
	};		
    }

    async fn event_handler(&self) {	
	info!("DynamoDB is running");
	let mut rx = self.rx.lock().await;	
        while let Some(job) = rx.next().await {
            let sender = job.sender;
            match job.job {
                PersistenceJobType::Store(value) => {
		    self.store(value,sender);
                },

		PersistenceJobType::Retrieve(value)=>{
		    self.retrieve(value,sender);
		},
	    }
	}
    }
}

#[async_trait]
impl<'l> Store for DynamoDbStore<'l> {
    async fn configure_store(&self){
	info!("Configuring DynamoDB store {:?} ", self);
        let f1 = async { self.event_handler().await };
        let _res = f1.await;
    }
}



