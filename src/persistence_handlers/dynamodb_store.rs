use std::collections::HashMap;
use rusoto_dynamodb;
use rusoto_dynamodb::{DynamoDb,DynamoDbClient};
use async_trait::async_trait;
use tokio::sync::mpsc;
use crate::utils::config;
use crate::utils::structs::{RestToPersistenceContext,PersistenceJobType,StoreRequest,RetrieveRequest,PersistenceResult,BackendStatusCodes,DeleteRequest};
use crate::persistence_handlers::Store;
use tokio::stream::StreamExt;
use futures::channel::oneshot::Sender;
use std::sync::Arc;
use futures::lock::Mutex;
use std::str::FromStr;
use bytes::Bytes;


#[derive(Debug)]
pub struct DynamoDbStore<'l>{
    config:&'l config::DynamoDb,
    rx: Arc<Mutex<mpsc::Receiver<RestToPersistenceContext>>>
}


impl<'l> DynamoDbStore<'l>{    
    pub fn new(config:&'l config::DynamoDb,rx: Arc<Mutex<mpsc::Receiver<RestToPersistenceContext>>>)->Self{
	DynamoDbStore{
	    config,
	    rx
	}	
    }
    async fn store(&self, client: &DynamoDbClient, sr:StoreRequest,sender:Sender<PersistenceResult>){
	let data_attr = rusoto_dynamodb::AttributeValue{
	    b:Some(Bytes::from(sr.payload.clone())),
	    .. Default::default()			
	};

	let key_attr = rusoto_dynamodb::AttributeValue{
	    s:Some(sr.key.clone()),
	    .. Default::default()			
	};
	let mut item = HashMap::new();
	item.insert("key".to_string(),key_attr);
	item.insert("data".to_string(), data_attr);


	let put_item_input = rusoto_dynamodb::PutItemInput{
	    table_name:sr.table_name.clone(),
	    item,
	    return_values:Some("ALL_OLD".to_string()),
	    .. Default::default()			    			    			    			    
	};	
	
	let put_item_output = client.put_item(put_item_input).await;
	let pr = match put_item_output{
	    Ok(output)=>{
		let payload = match output.attributes {
		    Some(item)=>{
			let maybe_data_attr = item.get("data");
			match maybe_data_attr{
			    Some(data_attr)=>{
				match data_attr.b.clone(){
				    Some(data)=>{
					data.to_vec()
				    },
				    None=>{
					vec![]				
				    }
				}
			    },
			    None=>{
				vec![]
			    }
			}
			
		    },
		    None =>{
			vec![]
		    }
		};
		
		PersistenceResult{
		    correlation_id: sr.correlation_id,
		    status: BackendStatusCodes::Ok("DynamoDb is good".to_string()),
		    payload: payload
		}
	    },
	    Err(e)=>{
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

    async fn retrieve(&self, client: &DynamoDbClient, rr:RetrieveRequest,sender:Sender<PersistenceResult>){
	let key_attr = rusoto_dynamodb::AttributeValue{
	    s:Some(rr.key.clone()),
	    .. Default::default()			
	};

	let mut key = HashMap::new();
	key.insert("key".to_string(),key_attr);
	
	let get_item_input = rusoto_dynamodb::GetItemInput{
	    table_name:rr.table_name.clone(),
	    key,
	    .. Default::default()			    			    			    			    
	};	
	let get_item_output = client.get_item(get_item_input).await;
	    	    		
	let gr = match get_item_output{
	    Ok(output)=>{		
		let payload = match output.item {
		    Some(item)=>{
			let maybe_data_attr = item.get("data");
			match maybe_data_attr{
			    Some(data_attr)=>{
				match data_attr.b.clone(){
				    Some(data)=>{
					data.to_vec()
				    },
				    None=>{
					vec![]				
				    }
				}
			    },
			    None=>{
				vec![]
			    }
			}
			
		    },
		    None =>{
			vec![]
		    }
		};
		
		PersistenceResult{
		    correlation_id: rr.correlation_id,
		    status: BackendStatusCodes::Ok("DynamoDb is good".to_string()),
		    payload
		}
	    },
	    Err(e)=>{
		PersistenceResult{
		    correlation_id: rr.correlation_id,
		    status: BackendStatusCodes::Error(e.to_string()),
		    payload: vec![]
		}
	    }
	};
	
	let r = sender.send(gr);
	if r.is_err() {
	    warn!("Can't send response {:?}",r);
	};		
    }

    async fn delete(&self, client: &DynamoDbClient, rr:DeleteRequest,sender:Sender<PersistenceResult>){
	let key_attr = rusoto_dynamodb::AttributeValue{
	    s:Some(rr.key.clone()),
	    .. Default::default()			
	};

	let mut key = HashMap::new();
	key.insert("key".to_string(),key_attr);
	
	let delete_item_input = rusoto_dynamodb::DeleteItemInput{
	    table_name:rr.table_name.clone(),
	    return_values:Some("ALL_OLD".to_string()),
	    key,
	    .. Default::default()			    			    			    			    
	};	
	let delete_item_output = client.delete_item(delete_item_input).await;
	    	    		
	let gr = match delete_item_output{
	    Ok(output)=>{		
		let payload = match output.attributes {
		    Some(item)=>{
			let maybe_data_attr = item.get("data");
			match maybe_data_attr{
			    Some(data_attr)=>{
				match data_attr.b.clone(){
				    Some(data)=>{
					data.to_vec()
				    },
				    None =>{
					vec![]				
				    }
				}
			    },
			    None =>{
				vec![]
			    }
			}
			
		    },
		    None =>{
			vec![]
		    }
		};
		
		PersistenceResult{
		    correlation_id: rr.correlation_id,
		    status: BackendStatusCodes::Ok("DynamoDb is good".to_string()),
		    payload
		}
	    },
	    Err(e)=>{
		PersistenceResult{
		    correlation_id: rr.correlation_id,
		    status: BackendStatusCodes::Error(e.to_string()),
		    payload: vec![]
		}
	    }
	};
	
	let r = sender.send(gr);
	if r.is_err() {
	    warn!("Can't send response {:?}",r);
	};		
    }

    async fn event_handler(&self) {
	let region = if let Ok(region) = rusoto_signature::Region::from_str(&self.config.region){
	    region
	}else{
	    warn!("Unknown region {}",self.config.region);
	    return;
	};
	
	let client = rusoto_dynamodb::DynamoDbClient::new(region);
	
	info!("DynamoDB is running");
	let mut rx = self.rx.lock().await;	
        while let Some(job) = rx.next().await {
            let sender = job.sender;
            match job.job {
                PersistenceJobType::Store(value) => {
		    self.store(&client, value,sender).await;
                },

		PersistenceJobType::Retrieve(value)=>{
		    self.retrieve(&client, value,sender).await;
		},
		
		PersistenceJobType::Delete(value)=>{
		    self.delete(&client, value,sender).await;
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



