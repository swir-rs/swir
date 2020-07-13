FROM gradle:jre14
COPY build.gradle ./
COPY settings.gradle ./
COPY src ./src
COPY grpc_api ./grpc_api
RUN gradle --no-daemon installDist
EXPOSE 50051 50052
ENTRYPOINT ["build/install/swir-grpc-sink/bin/swir-grpc-sink"]