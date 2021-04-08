use crate::persistence_handlers::Store;
use crate::utils::{
    config::{ClientToBackendDatabaseResolver, Redis},
    metric_utils::OutMetricInstruments,
    structs::*,
};

use async_trait::async_trait;
use opentelemetry::KeyValue;
use redis::{pipe, Client, Commands, Connection};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::{mpsc, oneshot::Sender, Mutex};
use tracing::info_span;

#[derive(Debug)]
pub struct RedisStore {
    config: Redis,
    rx: Arc<Mutex<mpsc::Receiver<RestToPersistenceContext>>>,
    metrics: Arc<OutMetricInstruments>,
}

impl RedisStore {
    pub fn new(config: Redis, rx: Arc<Mutex<mpsc::Receiver<RestToPersistenceContext>>>, metrics: Arc<OutMetricInstruments>) -> Self {
        RedisStore { config, rx, metrics }
    }

    fn validate_mapping(&self, pr: &dyn PersistenceRequest) -> Option<String> {
        let maybe_table_name = self.config.get_backend_table_name_for_client_table_name(&pr.get_table_name());
        if let Some(table_name) = maybe_table_name {
            Some(table_name)
        } else {
            None
        }
    }

    fn store(&self, connection: &mut Connection, sr: StoreRequest, sender: Sender<PersistenceResult>) {
        let maybe_mapping = self.validate_mapping(&sr);
        let pr = match maybe_mapping {
            None => PersistenceResult {
                correlation_id: sr.get_correlation_id(),
                status: BackendStatusCodes::Error(format!("No mapping for {}", sr.get_table_name())),
                payload: vec![],
            },
            Some(_table_name) => {
                let res: Result<(Option<String>, String), redis::RedisError> = pipe().atomic().get(&sr.key).set(&sr.key, sr.payload).query(connection);
                info!("{:?}", res);
                match res {
                    Ok((r1, r2)) => {
                        let payload = if let Some(data) = r1 { data.into_bytes() } else { vec![] };
                        let status = if r2 == "OK" {
                            BackendStatusCodes::Ok("REDIS is good".to_string())
                        } else {
                            BackendStatusCodes::Ok(format!("Problem when storing key: {}", r2))
                        };

                        PersistenceResult {
                            correlation_id: sr.correlation_id,
                            status,
                            payload,
                        }
                    }

                    Err(e) => PersistenceResult {
                        correlation_id: sr.correlation_id,
                        status: BackendStatusCodes::Error(e.to_string()),
                        payload: vec![],
                    },
                }
            }
        };
        let r = sender.send(pr);
        if r.is_err() {
            warn!("Can't send response {:?}", r);
        };
    }

    fn retrieve(&self, connection: &mut Connection, sr: RetrieveRequest, sender: Sender<PersistenceResult>) {
        let maybe_mapping = self.validate_mapping(&sr);
        let pr = match maybe_mapping {
            None => PersistenceResult {
                correlation_id: sr.get_correlation_id(),
                status: BackendStatusCodes::Error(format!("No mapping for {}", sr.get_table_name())),
                payload: vec![],
            },
            Some(_table_name) => {
                let r: Result<String, redis::RedisError> = connection.get(sr.key);
                match r {
                    Ok(data) => PersistenceResult {
                        correlation_id: sr.correlation_id,
                        status: BackendStatusCodes::Ok("REDIS is good".to_string()),
                        payload: data.into_bytes(),
                    },

                    Err(e) => PersistenceResult {
                        correlation_id: sr.correlation_id,
                        status: BackendStatusCodes::Error(e.to_string()),
                        payload: vec![],
                    },
                }
            }
        };

        let r = sender.send(pr);
        if r.is_err() {
            warn!("Can't send response {:?}", r);
        };
    }

    fn delete(&self, connection: &mut Connection, sr: DeleteRequest, sender: Sender<PersistenceResult>) {
        let maybe_mapping = self.validate_mapping(&sr);
        let pr = match maybe_mapping {
            None => PersistenceResult {
                correlation_id: sr.get_correlation_id(),
                status: BackendStatusCodes::Error(format!("No mapping for {}", sr.get_table_name())),
                payload: vec![],
            },
            Some(_table_name) => {
                let res: Result<(Option<String>, i32), redis::RedisError> = pipe().atomic().get(&sr.key).del(&sr.key).query(connection);
                info!("{:?}", res);
                match res {
                    Ok((r1, r2)) => {
                        let payload = if let Some(data) = r1 { data.into_bytes() } else { vec![] };
                        let status = BackendStatusCodes::Ok(format!("Deleted keys : {:?}", r2));

                        PersistenceResult {
                            correlation_id: sr.correlation_id,
                            status,
                            payload,
                        }
                    }

                    Err(e) => PersistenceResult {
                        correlation_id: sr.correlation_id,
                        status: BackendStatusCodes::Error(e.to_string()),
                        payload: vec![],
                    },
                }
            }
        };
        let r = sender.send(pr);
        if r.is_err() {
            warn!("Can't send response {:?}", r);
        };
    }

    async fn event_handler(&self) {
        let client = Client::open(self.config.nodes[0].clone()).unwrap();
        let mut connection = client.get_connection().unwrap();

        info!("Redis is running");
        let mut rx = self.rx.lock().await;

        let counters = self.metrics.outgoing_counters.clone();
        let mut labels = self.metrics.labels.clone();
        let histograms = self.metrics.outgoing_histograms.clone();

        while let Some(ctx) = rx.recv().await {
            let parent_span = ctx.span;
            let span = info_span!(parent: &parent_span, "REDIS");
            let _s = span.enter();
            let sender = ctx.sender;
            let request_start = SystemTime::now();
            match ctx.job {
                PersistenceJobType::Store(value) => {
                    labels.push(KeyValue::new("operation", "store"));
                    counters.request_counter.add(1, &labels);
                    self.store(&mut connection, value, sender);
                }

                PersistenceJobType::Retrieve(value) => {
                    labels.push(KeyValue::new("operation", "retrieve"));
                    counters.request_counter.add(1, &labels);
                    self.retrieve(&mut connection, value, sender);
                }

                PersistenceJobType::Delete(value) => {
                    labels.push(KeyValue::new("operation", "delete"));
                    counters.request_counter.add(1, &labels);
                    self.delete(&mut connection, value, sender);
                }
            }
            histograms.request_response_time.record(request_start.elapsed().map_or(0.0, |d| d.as_secs_f64()), &labels);
        }
    }
}

#[async_trait]
impl Store for RedisStore {
    async fn configure_store(&self) {
        info!("Configuring Redis store {:?} ", self);
        let f1 = async { self.event_handler().await };
        f1.await;
    }
}
