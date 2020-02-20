FROM openjdk:14
ADD build/distributions/swir-grpc-sink.tar /
EXPOSE 50051 50052
ENTRYPOINT ["/swir-grpc-sink/bin/swir-grpc-sink"]