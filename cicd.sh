#necessary certs for HTTPs but only on the first run
#./generate-cert.sh



#java based components
cd clients/swir-java-client
./gradlew bootJar
docker build --tag swir-java-client:v2 .

cd ../swir-kafka-sink
./gradlew bootJar
docker build --tag swir-kafka-sink:v2 .

cd ../swir-nats-sink
./gradlew bootJar
docker build --tag swir-nats-sink:v2 .
cd ../../

cargo build --release --features="with_nats"
#cargo build --release
docker build . --build-arg executable=target/release/rustycar --build-arg client=clients/swir-java-client/build/libs/swir-java-client-0.0.1-SNAPSHOT.jar --build-arg swir_config=swir_docker.yaml -t swir:v2

## Remove if exists
docker-compose -f docker/docker-compose-swir.yml down --remove-orphans

# this should deploy the infrastructure
# Docker instance names/network name created by docker compose could change
docker-compose -f docker/docker-compose-infr.yml up -d

docker exec -t docker_kafka_1 kafka-topics.sh --bootstrap-server :9094 --create --topic Request --partitions 2 --replication-factor 1
docker exec -t docker_kafka_1 kafka-topics.sh --bootstrap-server :9094 --create --topic Response --partitions 2 --replication-factor 1
docker exec -t docker_kafka_1 kafka-topics.sh --bootstrap-server :9094 --create --topic RequestNoSidecar --partitions 2 --replication-factor 1
docker exec -t docker_kafka_1 kafka-topics.sh --bootstrap-server :9094 --create --topic ResponseNoSidecar --partitions 2 --replication-factor 1

# this should deploy swir and other components
docker-compose  -f docker/docker-compose-swir.yml up -d

#use these to produce and receive messasges

#docker run --network docker_swir-net -it curlimages/curl -v -d '{"endpoint":{"url":"http://docker_swir_1:8090/response"},"client_topic":"SubscribeToAppA"}' -H "Content-Type: application/json" -X POST http://docker_swir_1:8080/subscribe
#docker run --network docker_swir-net -it curlimages/curl -v -d '{"endpoint":{"url":"http://docker_swir_1:8090/response"},"client_topic":"SubscribeToAppB"}' -H "Content-Type: application/json" -X POST http://docker_swir_1:8080/subscribe

docker run --network docker_swir-net -it --rm curlimages/curl -v -d '{"endpoint":{"url":"http://docker_swir-java-client_1:8090/response"},"client_topic":"SubscribeToAppA"}' -H "Content-Type: application/json" -X POST http://docker_swir_1:8080/subscribe
docker run --network docker_swir-net -it --rm curlimages/curl -v -d '{"endpoint":{"url":"http://docker_swir-java-client_1:8090/response"},"client_topic":"SubscribeToAppB"}' -H "Content-Type: application/json" -X POST http://docker_swir_1:8080/subscribe



#Kafka test
#docker run --network docker_swir-net -it --rm curlimages/curl -v -d '{"messages":100, "threads":4, "sidecarUrl":"http://docker_swir_1:8080","clientTopic":"ProduceToAppA"}' -H "Content-Type: application/json" -X POST http://docker_swir-java-client_1:8090/test
#Nats test
#docker run --network docker_swir-net -it --rm curlimages/curl -v -d '{"messages":100, "threads":4, "sidecarUrl":"http://docker_swir_1:8080","clientTopic":"ProduceToAppA"}' -H "Content-Type: application/json" -X POST http://docker_swir-java-client_1:8090/test


#docker cp docker_swir_1:/pcap.logs ~/Workspace/rustycar/
#docker-compose -f docker/docker-compose-swir.yml down --remove-orphans
#docker logs docker_swir_1 > logs 2>&1