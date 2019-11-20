FROM openjdk:14-jdk-alpine AS KafkaSink
#VOLUME /tmp
#ARG SPRINGBOOT_APP=build/libs/KafkaSink-0.1.0-SNAPSHOT.jar
#COPY ${SPRINGBOOT_APP} app.jar
COPY build/libs/KafkaSink-0.0.1-SNAPSHOT.jar /app.jar
ENTRYPOINT ["java","-Djava.security.egd=file:/dev/./urandom","-Dspring.profiles.active=docker","-jar","/app.jar"]
