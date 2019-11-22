cd clients/swir-java-client
./gradlew bootJar
docker build --tag swir-java-client .

cd ../swir-kafka-sink
./gradlew bootJar
docker build --tag swir-kafka-sink .

cd ../kafka-java-client
./gradlew bootJar
docker build --tag kafka-java-client .
cd ../../

cargo build --release --target-dir target/with_kafka
cargo build --release --features="with_nats" --target-dir target/with_nats
docker build . --build-arg executable=target/with_kafka/release/rustycar --build-arg client=clients/swir-java-client/build/libs/swir-java-client-0.0.1-SNAPSHOT.jar -t swir:with_kafka
docker build . --build-arg executable=target/with_nats/release/rustycar --build-arg client=clients/swir-java-client/build/libs/swir-java-client-0.0.1-SNAPSHOT.jar -t swir:with_nats



docker-compose -f docker/docker-compose-infr.yml up -d

docker exec -t docker_kafka_1 kafka-topics.sh --bootstrap-server :9094 --create --topic Request --partitions 2 --replication-factor 1
docker exec -t docker_kafka_1 kafka-topics.sh --bootstrap-server :9094 --create --topic Response --partitions 2 --replication-factor 1

docker-compose -f docker/docker-compose-swir.yml up -d


#
#docker run --network docker_swir-net -it curlimages/curl -v -d '{"endpoint":{"url":"http://docker_swir-java-client_1:8090/response"}}' -H "Content-Type: application/json" -X POST http://docker_swir_1:8080/subscribe
#docker run --network docker_swir-net -it curlimages/curl -v -d '{"messages":1000, "threads":2, "sidecarUrl":"http://docker_swir_1:8080/publish"}' -H "Content-Type: application/json" -X POST http://docker_swir-java-client_1:8090/test
#docker run --network docker_swir-net -it curlimages/curl -v -d '{"messages":10000, "threads":4, "sidecarUrl":"http://docker_swir_1:8080/publish"}' -H "Content-Type: application/json" -X POST http://docker_kafka-java-client_1:8091/test