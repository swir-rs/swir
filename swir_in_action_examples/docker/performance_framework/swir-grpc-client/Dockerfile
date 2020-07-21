FROM gradle:jre14
COPY build.gradle ./
COPY settings.gradle ./
COPY src ./src
COPY grpc_api ./grpc_api
RUN gradle --no-daemon  installDist
EXPOSE 50051
ENTRYPOINT ["build/install/swir-grpc-client/bin/swir-grpc-client"]