FROM maven:3.6-slim
COPY src ./src
COPY pom.xml ./
RUN mvn package
ENTRYPOINT ["java","-Djava.security.egd=file:/dev/./urandom","-jar","target/swir-nats-sink-1.0-SNAPSHOT-jar-with-dependencies.jar"]
