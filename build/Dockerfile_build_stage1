FROM rust:1.50 as swir_builder
WORKDIR /swir

COPY build.rs ./
COPY rustfmt.toml ./
COPY deny.toml ./
COPY grpc_api ./grpc_api
COPY Cargo.toml ./
COPY Cargo.lock ./
RUN rustup component add rustfmt 
RUN cargo fetch
COPY src ./src

RUN cargo build --release --all-features