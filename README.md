# SWIR or Sidecar Written in Rust


![Logo](graphics/swir_logo.png)


For alternative meaning of [SWIR](https://en.pons.com/translate/polish-english/swir)


## Rationale

In an enterprise world where many teams are using different technology stacks, the introduction of sidecar could offer an avenue to provide a uniform set of capabilities to all applications irrespective of which language their ware implemented.
Sidecar approach provides a necessary abstraction layer that protects the application teams from the underlying infrastructure.
The applications implementing business logic stay relatively simple, and the sidecar takes on responsibility for providing consistent and the most optimal way to utilise available resources.
This approach can produce better division of responsibilities and thus better organisational alignment of teams, particularly in the large enterprise environment. Application teams can focus on delivering business requirements. The infrastructure support teams can focus on providing the necessary rails on which the application can run.  

The rationale of this point could be explained better with a simple stakeholder analysis.

- **Application teams** - Many application teams can solely focus on business logic and worry less about dependency management and infrastructure on which the application is running. Since a sidecar is a separate process, the main application becomes more streamlined as most of the dependencies are part of a sidecar. Application owners don't need to worry about how to communicate with other parts of the solutions since the sidecar provides the necessary functionality. The fact that the sidecar sits between the business logic and the infrastructure means that the business flows in the entire solution could potentially be tested with relative ease on a developer's laptop with sidecar taking responsibility for mocking the production environment. As enterprise solutions grow in size and complexity, commonalities emerge which are prime candidates for offloading to a sidecar. Moving boilerplate code to the sidecar ensures that the necessary implementation is done in one place and in the best possible way.  Since all components of the solution rely on a sidecar, automatically these components have access to the same functionality.

- **Infrastructure teams** - With a sidecar approach, the infrastructure team can safely transition the whole enterprise to the next best thing without forcing the application teams to re-design or re-build their business logic. Applications implementing business logic are unaffected by a potential transition from one type of broker to a different kind of broker or perhaps ditching brokers altogether in favour of client-side load balancing with gRPC.  The abstraction layer provided by a sidecar enables application teams to express solution topology in infrastructure-independent terms which then could be translated into concepts specific to the infrastructure in a given environment. The additional separation between business logic and configuration should enable smoother and less complex deployments.       

- **Operations and support teams** - The solutions built from sidecar enabled applications could achieve consistency at the enterprise level when it comes to logging in monitoring since all sidecars would adhere to the same logging principles. A sidecar could easily inject information needed to trace requests as they traverse the solution. One could envisage, the operations teams building the whole solution with some mock or rudimentary business logic to ensure that all blocks are wired together correctly. In this sense, the sidecar approach allows application, infrastructure and operation teams to move at different speeds without impacting each other's schedule. The teams can move quicker with testing, and the application and infrastructure can initially be tested in splendid isolation since sidecar makes the hard interdependencies less impacting.

- **Corporate security teams** - From the security teams perspective, the sidecar approach should be particularly appealing. Instead of having to vet many different technology stacks with many different ways of securing network connectivity or encrypting data, only a sidecar needs to be thoroughly vetted. Adherence to security principles could be governed and enforced at the enterprise level through changes to the sidecar and without impacting the schedule or functionality of the business logic. Common but complex things such as the use of encryption could be accessible to applications written in different technology stacks through a simple API call to a sidecar. In a braver scenario, a specialised sidecar could encrypt all highly sensitive fields before being written to persistent storage. Again simplifying the life for the application and security teams and helping the enterprise building a better and more secure solution.

### Similar Frameworks

SWIR has been influenced by Microsoft's [Distributed Application Runtime - Dapr](https://github.com/dapr/dapr).

It is hard to compete with Microsoft's unlimited resources, but someday perhaps SWIR might achieve a parity :)


## Rust
Rust is a safe language, and side by side benchmarks show that the applications which are written in Rust achieve performance comparable with applications written in C or C++. In choosing an implementation language for a sidecar, these two factors are probably the most important. Rust language secure design guarantees that an attacker can't compromise the sidecar due to problems with memory safety. At the same time, since sidecar is responsible for most of the application's system-level functionality, it is crucial to minimise sidecar's impact on the performance. As Rust has no runtime nor garbage collector, it can run very fast and with small latency.

### Similar Frameworks

SWIR has been influenced by Microsoft's [Distributed Application Runtime - Dapr](https://github.com/dapr/dapr).

It is hard to compete with Microsoft's unlimited resources, but someday perhaps SWIR might achieve a parity :)


## Rust
Rust is a safe language, and side by side benchmarks show that the applications which are written in Rust achieve performance comparable with applications written in C or C++. In choosing an implementation language for a sidecar, these two factors are probably the most important. Rust language secure design guarantees that an attacker can't compromise the sidecar due to problems with memory safety. At the same time, since sidecar is responsible for most of the application's system-level functionality, it is crucial to minimise sidecar's impact on the performance. As Rust has no runtime nor garbage collector, it can run very fast and with small latency.


## About Swir
This project is just a starting point to a conversation about sidecars, particularly for solutions consisting of many event-driven components. Even then it has some interesting features mainly because of the quality of crates created and maintained by Rust community:
SWIR:
 - has moved to asynchronous programming
 - uses [Hyper](https://hyper.rs/) to expose REST interfaces over HTTP or HTTPS
 - uses [Tonic](https://docs.rs/tonic/0.1.1/tonic/index.html) to handle gRPC calls
 - uses [rdkafka](https://github.com/fede1024/rust-rdkafka) to talk to [Kafka](https://kafka.apache.org/) brokers
 - uses [nats](https://github.com/jedisct1/rust-nats) to talk to [NATS](https://nats.io) brokers
 - is using modified [config-rs](https://github.com/swir-rs/config-rs) so various aspects can be configured via a yaml file and environment variables can be easily injected based on an environment
 - SWIR uses conditional compilation which allows creating sidecars with just Kafka or Kafka and NATS
 - SpringBoot and gRPC Java clients and other components allowing testing it end to end
 - SWIR can start the client application (for time being only SpringBoot standalone jars)


## Short Term Roadmap
- gRPC service meshing at the backend
- encryption offloading
- Redis/Cassandra facade
- logging

## Top Level Architecture

![Diagram](./graphics/swir.png)

# Requirements
- To compile you will need cargo 1.39.0
- Linux Ubuntu or similar.
- Docker and Docker compose to run the infrastructure and the examples
- Java 1.8 or higher
- Gradle
- openssl to generate certs if you want to enable HTTPs


# Running
Most of the steps are documented in [cicd.sh](cicd.sh) file.
```shell script
# generate necessary certs for HTTPs but only on the first run
# this requires openssl
./generate-cert.sh

# java based clients and sinks
cd clients/swir-java-client
./gradlew clean bootJar
docker build --tag swir-java-client:v2 .

cd ../swir-kafka-sink
./gradlew clean bootJar
docker build --tag swir-kafka-sink:v2 .

cd ../swir-nats-sink
./gradlew clean bootJar
docker build --tag swir-nats-sink:v2 .

cd ../swir-grpc-client
./gradlew clean build installDist assembleDist
docker build --tag swir-grpc-client:v2 .

cd ../swir-grpc-sink
./gradlew clean build installDist assembleDist
docker build --tag swir-grpc-sink:v2 .
cd ../../

# building SWIR
cargo build --release --features="with_nats"

# ideally the operating system use to build a SWIR binary should be the same as specified in Dockerfile
docker build . --build-arg executable=target/release/rustycar --build-arg client=clients/swir-java-client/build/libs/swir-java-client-0.0.1-SNAPSHOT.jar --build-arg swir_config=swir_docker.yaml -t swir:v2

## cleaning Docker enrironment just in caseRemove if exists
docker-compose -f docker/docker-compose-swir.yml down --remove-orphans

# deploying infrastructure (Kafka,Zookeeper,Nats)
# docker instance names/network name created by docker compose could change
docker-compose -f docker/docker-compose-infr.yml up -d

# creating topics
docker exec -t docker_kafka_1 kafka-topics.sh --bootstrap-server :9094 --create --topic Request --partitions 2 --replication-factor 1
docker exec -t docker_kafka_1 kafka-topics.sh --bootstrap-server :9094 --create --topic Response --partitions 2 --replication-factor 1
docker exec -t docker_kafka_1 kafka-topics.sh --bootstrap-server :9094 --create --topic RequestNoSidecar --partitions 2 --replication-factor 1
docker exec -t docker_kafka_1 kafka-topics.sh --bootstrap-server :9094 --create --topic ResponseNoSidecar --partitions 2 --replication-factor 1

# deploying SWIR, clients and sinks
docker-compose  -f docker/docker-compose-swir.yml up -d

# running test client over REST to Kafka

docker run --network docker_swir-net -it --rm curlimages/curl -v -d '{"endpoint":{"url":"http://docker_swir-java-client_1:8090/response"},"client_topic":"SubscribeToAppA"}' -H "Content-Type: application/json" -X POST http://docker_swir_1:8080/subscribe
docker run --network docker_swir-net -it --rm curlimages/curl -v -d '{"messages":100000, "threads":10, "sidecarUrl":"http://docker_swir_1:8080","clientTopic":"ProduceToAppA","missedPackets":50}' -H "Content-Type: application/json" -X POST http://docker_swir-java-client_1:8090/test

# running test client over REST to Nats
docker run --network docker_swir-net -it --rm curlimages/curl -v -d '{"endpoint":{"url":"http://docker_swir-java-client_1:8090/response"},"client_topic":"SubscribeToAppB"}' -H "Content-Type: application/json" -X POST http://docker_swir_1:8080/subscribe
docker run --network docker_swir-net -it --rm curlimages/curl -v -d '{"messages":100000, "threads":10, "sidecarUrl":"http://docker_swir_1:8080","clientTopic":"ProduceToAppB","missedPackets":50}' -H "Content-Type: application/json" -X POST http://docker_swir-java-client_1:8090/test

# running test client over gRPC to Kafka
docker run -ti --network docker_swir-net  --rm -e sidecar_hostname=swir -e sidecar_port=50051 -e messages=100000 -e threads=10 -e client_request_topic=ProduceToAppA -e client_response_topic=SubscribeToAppA  swir-grpc-client:v2
# running test client over gRPC to Nats
docker run -ti --network docker_swir-net  --rm -e sidecar_hostname=swir -e sidecar_port=50051 -e messages=100000 -e threads=10 -e client_request_topic=ProduceToAppB -e client_response_topic=SubscribeToAppB  swir-grpc-client:v2


```
