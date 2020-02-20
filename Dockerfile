FROM ubuntu:19.10
COPY demoCA /demoCA
COPY swir.CA.pem /swir.CA.pem
RUN apt-get update && apt-get install -y openjdk-14-jdk
ARG executable
ARG client
ARG swir_config
COPY ${executable} /swir
COPY $client /client.jar
COPY $swir_config /swir.yaml
RUN chmod +x /swir
ENV RUST_BACKTRACE=full
ENV RUST_LOG=info
EXPOSE 8080 8443 8090 50051
ENTRYPOINT ["./swir"]
