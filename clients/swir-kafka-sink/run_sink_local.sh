export bootstrap_servers=127.0.0.1:9092
export sink_consumer_topic=Response
export sink_consumer_group=swir
export sink_producer_topic=Request
./gradlew bootRun