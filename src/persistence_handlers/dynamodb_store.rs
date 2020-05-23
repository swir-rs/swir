use crate::persistence_handlers::Store;
use crate::utils::config;
use crate::utils::config::ClientToBackendDatabaseResolver;
use crate::utils::structs::{BackendStatusCodes, DeleteRequest, PersistenceJobType, PersistenceRequest, PersistenceResult, RestToPersistenceContext, RetrieveRequest, StoreRequest};
use async_trait::async_trait;
use bytes::Bytes;

use rusoto_dynamodb;
use rusoto_dynamodb::{DynamoDb, DynamoDbClient};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::stream::StreamExt;
use tokio::sync::{
    mpsc,
    Mutex,
    oneshot::Sender	
};

#[derive(Debug)]
pub struct DynamoDbStore {
    config: config::DynamoDb,
    rx: Arc<Mutex<mpsc::Receiver<RestToPersistenceContext>>>,
}

impl DynamoDbStore {
    pub fn new(config: config::DynamoDb, rx: Arc<Mutex<mpsc::Receiver<RestToPersistenceContext>>>) -> Self {
        DynamoDbStore { config, rx }
    }
    //     correlation_id: String, client_table_name: &str
    fn validate_mapping(&self, pr: &dyn PersistenceRequest) -> Option<String> {
        let maybe_table_name = self.config.get_backend_table_name_for_client_table_name(&pr.get_table_name());
        if let Some(table_name) = maybe_table_name {
            Some(table_name)
        } else {
            None
        }
    }

    async fn store(&self, client: &DynamoDbClient, sr: StoreRequest, sender: Sender<PersistenceResult>) {
        let maybe_mapping = self.validate_mapping(&sr);
        let pr = match maybe_mapping {
            None => PersistenceResult {
                correlation_id: sr.get_correlation_id(),
                status: BackendStatusCodes::Error(format!("No mapping for {}", sr.get_table_name())),
                payload: vec![],
            },
            Some(table_name) => {
                let data_attr = rusoto_dynamodb::AttributeValue {
                    b: Some(Bytes::from(sr.payload.clone())),
                    ..Default::default()
                };

                let key_attr = rusoto_dynamodb::AttributeValue {
                    s: Some(sr.key.clone()),
                    ..Default::default()
                };
                let mut item = HashMap::new();
                item.insert("partition_key".to_string(), key_attr);
                item.insert("data".to_string(), data_attr);

                let put_item_input = rusoto_dynamodb::PutItemInput {
                    table_name,
                    item,
                    return_values: Some("ALL_OLD".to_string()),
                    ..Default::default()
                };
                debug!("Store request => {:?}", put_item_input);

                let put_item_output = client.put_item(put_item_input).await;
                match put_item_output {
                    Ok(output) => {
                        let payload = match output.attributes {
                            Some(item) => {
                                let maybe_data_attr = item.get("data");
                                match maybe_data_attr {
                                    Some(data_attr) => match data_attr.b.clone() {
                                        Some(data) => data.to_vec(),
                                        None => vec![],
                                    },
                                    None => vec![],
                                }
                            }
                            None => vec![],
                        };

                        PersistenceResult {
                            correlation_id: sr.correlation_id,
                            status: BackendStatusCodes::Ok("DynamoDb is good".to_string()),
                            payload,
                        }
                    }
                    Err(e) => {
                        debug!("Can't store -> {:?}", e);
                        PersistenceResult {
                            correlation_id: sr.correlation_id,
                            status: BackendStatusCodes::Error(e.to_string()),
                            payload: vec![],
                        }
                    }
                }
            }
        };

        let r = sender.send(pr);
        if r.is_err() {
            warn!("Can't send response {:?}", r);
        };
    }

    async fn retrieve(&self, client: &DynamoDbClient, rr: RetrieveRequest, sender: Sender<PersistenceResult>) {
        let maybe_mapping = self.validate_mapping(&rr);
        let pr = match maybe_mapping {
            None => PersistenceResult {
                correlation_id: rr.get_correlation_id(),
                status: BackendStatusCodes::Error(format!("No mapping for {}", rr.get_table_name())),
                payload: vec![],
            },
            Some(table_name) => {
                let key_attr = rusoto_dynamodb::AttributeValue {
                    s: Some(rr.key.clone()),
                    ..Default::default()
                };

                let mut key = HashMap::new();
                key.insert("partition_key".to_string(), key_attr);

                let get_item_input = rusoto_dynamodb::GetItemInput {
                    table_name,
                    key,
                    ..Default::default()
                };
                let get_item_output = client.get_item(get_item_input).await;

                match get_item_output {
                    Ok(output) => {
                        let payload = match output.item {
                            Some(item) => {
                                let maybe_data_attr = item.get("data");
                                match maybe_data_attr {
                                    Some(data_attr) => match data_attr.b.clone() {
                                        Some(data) => data.to_vec(),
                                        None => vec![],
                                    },
                                    None => vec![],
                                }
                            }
                            None => vec![],
                        };

                        PersistenceResult {
                            correlation_id: rr.correlation_id,
                            status: BackendStatusCodes::Ok("DynamoDb is good".to_string()),
                            payload,
                        }
                    }
                    Err(e) => {
                        debug!("Can't retrieve -> {:?}", e);
                        PersistenceResult {
                            correlation_id: rr.correlation_id,
                            status: BackendStatusCodes::Error(e.to_string()),
                            payload: vec![],
                        }
                    }
                }
            }
        };

        let r = sender.send(pr);
        if r.is_err() {
            warn!("Can't send response {:?}", r);
        };
    }

    async fn delete(&self, client: &DynamoDbClient, rr: DeleteRequest, sender: Sender<PersistenceResult>) {
        let maybe_mapping = self.validate_mapping(&rr);
        let pr = match maybe_mapping {
            None => PersistenceResult {
                correlation_id: rr.get_correlation_id(),
                status: BackendStatusCodes::Error(format!("No mapping for {}", rr.get_table_name())),
                payload: vec![],
            },
            Some(table_name) => {
                let key_attr = rusoto_dynamodb::AttributeValue {
                    s: Some(rr.key.clone()),
                    ..Default::default()
                };

                let mut key = HashMap::new();
                key.insert("partition_key".to_string(), key_attr);

                let delete_item_input = rusoto_dynamodb::DeleteItemInput {
                    table_name,
                    return_values: Some("ALL_OLD".to_string()),
                    key,
                    ..Default::default()
                };
                let delete_item_output = client.delete_item(delete_item_input).await;

                match delete_item_output {
                    Ok(output) => {
                        let payload = match output.attributes {
                            Some(item) => {
                                let maybe_data_attr = item.get("data");
                                match maybe_data_attr {
                                    Some(data_attr) => match data_attr.b.clone() {
                                        Some(data) => data.to_vec(),
                                        None => vec![],
                                    },
                                    None => vec![],
                                }
                            }
                            None => vec![],
                        };

                        PersistenceResult {
                            correlation_id: rr.correlation_id,
                            status: BackendStatusCodes::Ok("DynamoDb is good".to_string()),
                            payload,
                        }
                    }
                    Err(e) => {
                        debug!("Can't delete -> {:?}", e);
                        PersistenceResult {
                            correlation_id: rr.correlation_id,
                            status: BackendStatusCodes::Error(e.to_string()),
                            payload: vec![],
                        }
                    }
                }
            }
        };

        let r = sender.send(pr);
        if r.is_err() {
            warn!("Can't send response {:?}", r);
        };
    }

    async fn event_handler(&self) {
        let region = if let Ok(region) = rusoto_signature::Region::from_str(&self.config.region) {
            region
        } else {
            warn!("Unknown region {}", self.config.region);
            return;
        };

        let client = rusoto_dynamodb::DynamoDbClient::new(region);

        info!("DynamoDB is running");
        let mut rx = self.rx.lock().await;
        while let Some(job) = rx.next().await {
            let sender = job.sender;

            match job.job {
                PersistenceJobType::Store(value) => {
                    self.store(&client, value, sender).await;
                }

                PersistenceJobType::Retrieve(value) => {
                    self.retrieve(&client, value, sender).await;
                }

                PersistenceJobType::Delete(value) => {
                    self.delete(&client, value, sender).await;
                }
            }
        }
    }
}

#[async_trait]
impl Store for DynamoDbStore {
    async fn configure_store(&self) {
        info!("Configuring DynamoDB store {:?} ", self);
        let f1 = async { self.event_handler().await };
        f1.await;
    }
}
