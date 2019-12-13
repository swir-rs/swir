FROM ubuntu:19.10
COPY demoCA /demoCA
COPY rustycar.CA.pem /rustycar.CA.pem
COPY swir-docker.yaml /swir.yaml
RUN apt-get update && apt-get install -y openjdk-14-jdk
ARG executable
ARG client
COPY ${executable} /rustycar
COPY $client /client.jar
RUN chmod +x /rustycar
ENV RUST_BACKTRACE=full
ENV RUST_LOG=info

EXPOSE 8080 8443 8090
ENTRYPOINT ["./rustycar"]