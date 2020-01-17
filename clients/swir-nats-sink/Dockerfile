FROM openjdk:14-jdk-alpine
COPY build/libs/swir-nats-sink-0.0.1-SNAPSHOT.jar /app.jar
ENTRYPOINT ["java","-Djava.security.egd=file:/dev/./urandom","-jar","/app.jar"]
