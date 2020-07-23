# SWIR in Action - Examples


* * *
### Requirements:
> **Docker** : 19.03.6

> **Minikube** : [1.11.0](https://github.com/kubernetes/minikube/releases/tag/v1.11.0)

> **Kubectl** : [1.17.0](https://storage.googleapis.com/kubernetes-release/release/v1.17.0/bin/linux/amd64/kubectl)

> **Helm** : [3.2.4](https://github.com/helm/helm/releases/tag/v3.2.4)

* * *

## Examples on how SWIR can benefit your organization dived by the environment type


### Docker and Docker Compose based examples. 
   - [PubSub, Store and Tracing with Kafka, Nats, Redis and Jaeger](docker/pubsub_and_store_example/README.md)
   - [Service Invocation and Tracing with Service Meshing, multicastDNS and Jaeger](docker/service_invocation_example/README.md)
   
### Kubernetes based examples. 

Running locally will require Docker, Minikube, Kubectl and Helm.

   - [PubSub, Store and Tracing with Kafka, Nats, Redis and Jaeger](kubernetes/pubsub_and_store_example/README.md)
   - [Service Invocation and Tracing with Service Meshing, multicastDNS and Jaeger](kubernetes/service_invocation_example/README.md)
   
### AWS based examples. 

Running SWIR in Amazon ECS. Docker is required to create images.

  - [PubSub and Store with ECS, Kinesis and DynamoDB](aws/pubsub_and_store_example/README.md)
  - [Service Invocation with Service Meshing, ECS, DynamoDB](aws/service_invocation_example/README.md)


  

