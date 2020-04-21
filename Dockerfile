FROM ubuntu:19.10
RUN apt-get update && apt-get upgrade -y && apt-get install libssl-dev libssl1.1 -y
COPY demoCA /demoCA
COPY swir.CA.pem /swir.CA.pem
ARG executable
ARG swir_config
COPY ${executable} /swir
COPY $swir_config /swir.yaml
RUN chmod +x /swir
ENV RUST_BACKTRACE=full
ENV RUST_LOG=info,rusoto_core=info,swir=debug,rusoto_dynamodb=info
EXPOSE 8080 8443 8090 50051
ENTRYPOINT ["./swir"]
