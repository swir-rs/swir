[![Build Status](https://travis-ci.com/swir-rs/swir.svg?branch=master)](https://travis-ci.com/swir-rs/swir)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![GitHub release](https://img.shields.io/github/release/swir-rs/swir.svg)](https://GitHub.com/Naereen/StrapDown.js/releases/)
[![Awesome Badges](https://img.shields.io/badge/badges-awesome-green.svg)](https://swir.rs)

# SWIR or Sidecar Written in Rust


![](graphics/swir_logo_sidecar.png)


For alternative meaning of [SWIR](https://en.pons.com/translate/polish-english/swir)

## Rationale
SWIR is a platform allowing you to build applications for the cloud and on-prem environments quickly by providing an abstraction layer between your business logic and the infrastructure. 

## How it works
SWIR exposes a set of high level APIs that your business logic component uses to tap into underlying resources such as PubSub, State, Service Invocation. SWIR API specification in OpenApi 3.0 format can be found [here](https://editor.swagger.io/?url=https://raw.githubusercontent.com/swir-rs/swir/master/open_api/client_api.yaml)

## Usecases 
Examples of SWIR in action for Docker, Kubernetes and AWS ECS are descibed [here](swir_in_action_examples/README.md)

## Top Level Architecture
![Diagram](./graphics/swir_architecture.png)


## Rust
Rust is a safe language, and side by side benchmarks show that the applications which are written in Rust achieve performance comparable with applications written in C or C++. In choosing an implementation language for a sidecar, these two factors are probably the most important. Rust language secure design guarantees that an attacker can't compromise the sidecar due to problems with memory safety. At the same time, since sidecar is responsible for most of the application's system-level functionality, it is crucial to minimise sidecar's impact on the performance. As Rust has no runtime nor garbage collector, it can run very fast and with small latency.


## About Swir
This project is just a starting point to a conversation about sidecars, particularly for solutions consisting of many event-driven components. Even then it has some interesting features mainly because of the quality of crates created and maintained by Rust community:
SWIR:
 - has moved to asynchronous programming
 - uses [Hyper](https://hyper.rs/) to expose REST interfaces over HTTP or HTTPS  
 - uses [Tonic](https://docs.rs/tonic/0.1.1/tonic/index.html) to handle gRPC calls  
 - uses [rdkafka](https://github.com/fede1024/rust-rdkafka) to talk to [Kafka](https://kafka.apache.org/) brokers  
 - uses [Nats](https://github.com/jedisct1/rust-nats) to talk to [NATS](https://nats.io) brokers  
 - uses [rusoto](https://github.com/rusoto/rusoto) AWS SDK for Rust   
 - uses [redis-rs](https://github.com/mitsuhiko/redis-rs) Redis SDK for Rust  
 - is using modified [config-rs](https://github.com/swir-rs/config-rs) so various aspects can be configured via a yaml file and environment variables can be easily injected based on an   environment  
 - adapted and improved mDNS(https://github.com/swir-rs/rust-mdns) to advertise/resolve services  
 - SWIR uses conditional compilation which allows creating sidecars with just Kafka or Kafka and NATS  
 - HTTP and gRPC Java and Python clients and other components allowing testing it end to end  

   
# Requirements
- Docker, Docker Compose to build the project, run the infrastructure and the examples. 
- Minikube to run examples for Kubernetes
- AWS account to run AWS based examples
- Rust 1.44.1 or above


### Similar Frameworks

SWIR has been influenced by Microsoft's [Distributed Application Runtime - Dapr](https://github.com/dapr/dapr). It is hard to compete with Microsoft's unlimited resources, but someday perhaps SWIR might achieve a parity :)
