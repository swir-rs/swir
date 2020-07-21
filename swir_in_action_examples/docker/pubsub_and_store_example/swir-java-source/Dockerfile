FROM maven:3.6-slim
COPY pom.xml ./
COPY src ./src
COPY grpc_api ./grpc_api
RUN mvn package
ENTRYPOINT ["mvn","exec:java"]