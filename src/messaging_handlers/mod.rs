use futures::future::join_all;
use sled::Db;

use async_trait::async_trait;

use crate::utils::config::{Channels, MemoryChannel};

mod kafka_handler;
#[cfg(feature = "with_nats")]
mod nats_handler;


#[async_trait]
trait Broker {
    async fn configure_broker(&self);
}


pub async fn configure_broker(messaging: Channels, db: Db, mc: MemoryChannel) {
    let mut brokers: Vec<Box<dyn Broker>> = vec!();
    let mut futures = vec!();

    let mut i = 0;
    for kafka in messaging.kafka {
        let mce = mc.kafka_memory_channels.get(i);
        i = i + 1;
        if let None = mce {
            continue;
        }
        let mce = mce.unwrap();

        let rx = mce.from_client_receiver.to_owned();
        let tx = mce.to_client_sender.to_owned();

        let mut consumer_topics = vec!();
        let mut consumer_groups = vec!();
        for ct in kafka.consumer_topics {
            consumer_topics.push(ct.consumer_topic);
            consumer_groups.push(ct.consumer_group);
        }

        let mut producer_topics = vec!();
        for pt in kafka.producer_topics {
            producer_topics.push(pt.producer_topic);
        }


        let kafka_broker = kafka_handler::KafkaBroker {
            broker_address: kafka.brokers,
            consumer_topics,
            consumer_groups,
            producer_topics,
            db: db.clone(),
            rx,
            tx,
        };
        brokers.push(Box::new(kafka_broker));
    }

    #[cfg(feature = "with_nats")] {
        for nats in messaging.nats {
            let mce = mc.nats_memory_channels.get(i);
            i = i + 1;
            if let None = mce {
                continue;
            }
            let mce = mce.unwrap();

            let rx = mce.from_client_receiver.to_owned();
            let tx = mce.to_client_sender.to_owned();

            let mut consumer_topics = vec!();
            let mut consumer_groups = vec!();
            for ct in nats.consumer_topics {
                consumer_topics.push(ct.consumer_topic);
                consumer_groups.push(ct.consumer_group);
            }

            let mut producer_topics = vec!();
            for pt in nats.producer_topics {
                producer_topics.push(pt.producer_topic);
            }

            let nats_broker = nats_handler::NatsBroker {
                broker_address: nats.brokers,
                consumer_topics,
                consumer_groups,
                producer_topics,
                db: db.clone(),
                rx,
                tx,
            };
            brokers.push(Box::new(nats_broker));
        }
    }


    for broker in brokers {
        let f = async move { broker.configure_broker().await };
        futures.push(f);
    }

    join_all(futures).await;
}
