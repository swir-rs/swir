FROM ubuntu:19.10
RUN apt-get update && apt-get upgrade -y && apt-get install ca-certificates libssl-dev libssl1.1 -y
ARG executable
ARG swir_config
COPY ${executable} /swir
RUN chmod +x /swir
COPY $swir_config /swir.yaml
ENV RUST_BACKTRACE=full
ENV RUST_LOG=info,rusoto_core=info,swir=debug,rusoto_dynamodb=info
EXPOSE 8080 8090 50051
ENTRYPOINT ["./swir"]
