# SWIR or Sidecar Written in Rust

##Rationale
In an enterprise world where many teams are using different technology stacks, the introduction of sidecar could offer an avenue to provide a uniform set of capabilities to all applications irrespective of which language their ware implemented. 
Sidecar approach provides a necessary abstraction layer that protects the application teams from the underlying infrastructure. 
The applications implementing business logic stay relatively simple, and the sidecar takes on responsibility for providing consistent and the most optimal way to utilise available resources. 
In the large enterprise environment, such an approach can offer organisational improvements resulting in more clearly defined teams responsibilities and better overall governance mainly when an enterprise solution consists of
  many different application blocks.  
   
The rationale of this point could be explained better with a simple stakeholder analysis 

- **Application teams** - Many application teams can solely focus on business logic and worry less about dependency management and infrastructure on which the application will be running. Since a sidecar is a separate process, the main application becomes more streamlined as most of the dependencies are part of a sidecar. Application owners don't need to worry on how to communicate with other parts of the solutions since the sidecar will provide the necessary functionality. The fact that the sidecar sits between the business logic and the infrastructure means that the business flows in the entire solution could potentially be tested with relative ease on a developer's laptop with sidecar taking responsibility for mocking the production environment. As enterprise solutions grow in size and complexity, commonalities emerge which are prime candidates for offloading to a sidecar. Moving boilerplate code to the sidecar ensures that the necessary implementation is done in one place and in the best possible way.  Since all components of the solution rely on a sidecar, automatically these components will have access to the same functionality. 
                 
- **Infrastructure teams** - With a sidecar approach the infrastructure team can safely transition the whole enterprise to the next best thing without forcing the application teams to re-design or
  re-build their business logic. Applications implementing business logic are unaffected by a potential transition from one type of broker to a different kind of broker or perhaps ditching brokers altogether in favour of client-side load balancing with gRPC.  The abstraction layer provided by a sidecar enables application teams to express solution topology in infrastructure-independent terms which then could be translated into concepts specific to the infrastructure in a given environment. The additional separation between business logic and configuration should enable smoother and less complex deployments.       
     
- **Operations and support teams** - The solutions built from sidecar enabled applications could achieve consistency at the enterprise level when it comes to logging in monitoring since all sidecars would adhere to the same logging principles. A sidecar could easily inject information needed to trace requests as they traverse the solution. One could envisage, the operations teams building the whole solution with some mock or rudimentary business logic to ensure that all blocks are wired together correctly. In this sense the sidecar approach allows application, infrastructure and operation teams to move at different speeds without impacting each other's schedule and large chunks of the solution could be kept tested in isolation without as the hard interdependencies less impacting. 

- **Corporate security teams** - From the security teams perspective, the sidecar approach should be particularly appealing. Instead of having to vet many different technology stacks with many different ways of securing network connectivity or encrypting data, sidecar will deal with most of these aspects and will isolate all complexity in one carefully vetted place. Adherence to security principles could be governed and enforced at the enterprise level through changes to the sidecar and without impacting the schedule or functionality of the business logic. Common but complex things such as the use of encryption could be accessible to applications written in different technology stacks through a simple API call to a sidecar. In a braver scenario, a specialised sidecar could encrypt all highly sensitive fields before being written to persistent storage. Again simplifying the life for the application and security teams and helping the enterprise building a better and more secure solution. 


##Rust
Rust is a safe language, and side by side benchmarks show that the applications which are written in Rust achieve performance comparable with applications written in C or C++. In choosing an implementation language for a sidecar, these two factors are probably the most important. Rust language secure design guarantees that an attacker can't compromise the sidecar due to problems with memory safety. At the same time, since sidecar is responsible for most of the application's system-level functionality, it is crucial to minimise sidecar's impact on the performance. As Rust has no runtime nor garbage collector, it can run very fast and with small latency.


##MVP (not even beta)
This project is just a starting point to a conversation about sidecars, particularly for solutions consisting of many event-driven components. Even then it has some interesting features mainly because of the quality of crates  created and maintained by Rust community:
 - SWIR uses [Hyper](https://hyper.rs/) to expose REST interfaces over HTTP or HTTPS 
 - SWIR uses [rdkafka](https://github.com/fede1024/rust-rdkafka) to talk to [Kafka](https://kafka.apache.org/) brokers
 - SWIR uses [natsclient](https://github.com/encabulators/natsclient) to talk to [NATS](https://nats.io) brokers
 - SWIR uses conditional compilation which allows creating sidecars with just NATS or just Kafka dependencies
 - SpringBoot Java client and other components allowing testing it end to end
 - SWIR can start the client application (for time being only SpringBoot standalone jars)   
 
##Short Term Roadmap
- gRPC based interfaces with [Tonic](https://github.com/hyperium/tonic)
- encryption offloading
- some example enterprise patterns as documented here https://www.enterpriseintegrationpatterns.com/

##How to use it

#Requirements
- To compile you will need cargo 1.39.0
- Linux Ubuntu or similar.
- Docker and Docker compose to run the infrastructure and the examples
- Java 1.8 or higher
- Gradle 
- openssl to generate certs if you want to enable HTTPs
 

#Running 
Most of the steps are documented in cicd.sh file.
```shell script
#necessary certs for HTTPs 
./generate-cert.sh

#java based components

cd clients/swir-java-client
./gradlew bootJar
docker build --tag swir-java-client .

cd ../swir-kafka-sink
./gradlew bootJar
docker build --tag swir-kafka-sink .

#optional component to test solution's performance without a sidecar
cd ../kafka-java-client
./gradlew bootJar
docker build --tag kafka-java-client .
cd ../../

cargo build --release --target-dir target/with_kafka
cargo build --release --features="with_nats" --target-dir target/with_nats


docker build . --build-arg executable=target/with_kafka/release/rustycar --build-arg client=clients/swir-java-client/build/libs/swir-java-client-0.0.1-SNAPSHOT.jar -t swir:with_kafka
docker build . --build-arg executable=target/with_nats/release/rustycar --build-arg client=clients/swir-java-client/build/libs/swir-java-client-0.0.1-SNAPSHOT.jar -t swir:with_nats

# this should deploy the infrastructure 
# Docker instance names/network name created by docker compose could change 
docker-compose -f docker/docker-compose-infr.yml up -d

docker exec -t docker_kafka_1 kafka-topics.sh --bootstrap-server :9094 --create --topic Request --partitions 2 --replication-factor 1
docker exec -t docker_kafka_1 kafka-topics.sh --bootstrap-server :9094 --create --topic Response --partitions 2 --replication-factor 1

# this should deploy swir and other components
docker-compose -f docker/docker-compose-swir.yml up -d

```

