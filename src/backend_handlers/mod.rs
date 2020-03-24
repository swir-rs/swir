use futures::future::join_all;
use async_trait::async_trait;
use crate::utils::config::{Channels, MemoryChannel};

mod client_handler;
mod kafka_handler;
mod aws_kinesis_handler;

#[cfg(feature = "with_nats")]
mod nats_handler;

#[async_trait]
trait Broker {
    async fn configure_broker(&self);
}


pub async fn configure_broker(messaging: Channels, mc: MemoryChannel) {
    let mut brokers: Vec<Box<dyn Broker>> = vec![];
    let mut futures = vec![];

    
    for (i,kafka) in messaging.kafka.into_iter().enumerate() {
        let mce = mc.kafka_memory_channels.get(i);
        if mce.is_none() {
            continue;
        }
        let mce = mce.unwrap();
        let rx = mce.from_client_receiver.to_owned();        
        let kafka_broker = kafka_handler::KafkaBroker::new(kafka,rx);
        brokers.push(Box::new(kafka_broker));
    }

    #[cfg(feature = "with_nats")]
    {

        for (i,nats) in messaging.nats.into_iter().enumerate() {
	    let mce = mc.nats_memory_channels.get(i);
            if mce.is_none() {
		continue;
            }
            let mce = mce.unwrap();
            let rx = mce.from_client_receiver.to_owned();
            let nats_broker = nats_handler::NatsBroker::new(nats,rx);
            brokers.push(Box::new(nats_broker));
        }
    }


    for (i,aws_kinesis) in messaging.aws_kinesis.into_iter().enumerate() {
        let mce = mc.aws_kinesis_memory_channels.get(i);
        if mce.is_none() {
            continue;
        }
        let mce = mce.unwrap();
        let rx = mce.from_client_receiver.to_owned();        
        let aws_kinesis_broker = aws_kinesis_handler::AwsKinesisBroker::new(aws_kinesis,rx);
        brokers.push(Box::new(aws_kinesis_broker));
    }

    debug!("Brokers to configure {}", brokers.len());
    for broker in brokers.iter() {
        let f = async move { broker.configure_broker().await };
        futures.push(f);
    }

    join_all(futures).await;
}
