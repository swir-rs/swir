FROM rust:1.50 as builder
WORKDIR /usr/src/swir
RUN rustup component add rustfmt 
COPY build.rs ./
COPY rustfmt.toml ./
COPY deny.toml ./
COPY grpc_api ./grpc_api
COPY Cargo.toml ./
COPY Cargo.lock ./
COPY src ./src
RUN cargo build --release --all-features


### Split into two files; one to build and one to actually run it
###https://docs.docker.com/develop/develop-images/multistage-build/

FROM debian:buster-slim
RUN apt-get update && apt-get upgrade -y && apt-get install -y extra-runtime-dependencies ca-certificates libssl-dev libssl1.1
COPY --from=builder /usr/src/swir/target/release/swir /swir
COPY $swir_config /swir.yaml
ENV RUST_BACKTRACE=full
ENV RUST_LOG=info,rusoto_core=info,swir=debug,rusoto_dynamodb=info
EXPOSE 8080 8090 50051
ENTRYPOINT ["./swir"]


#FROM ubuntu:19.10
#RUN apt-get update && apt-get upgrade -y && apt-get install ca-certificates libssl-dev libssl1.1 -y
#ARG executable
#ARG swir_config
#COPY ${executable} /swir
#RUN chmod +x /swir
#COPY $swir_config /swir.yaml
#ENV RUST_BACKTRACE=full
#ENV RUST_LOG=info,rusoto_core=info,swir=debug,rusoto_dynamodb=info
#EXPOSE 8080 8090 50051
#ENTRYPOINT ["./swir"]
