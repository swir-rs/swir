FROM gradle:jre14
COPY build.gradle ./
COPY settings.gradle ./
COPY src ./src
COPY grpc_api ./grpc_api
RUN gradle installDist

ENTRYPOINT ["build/install/swir-java-source/bin/swir-java-source"]