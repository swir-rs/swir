FROM openjdk:14-jdk-alpine
#VOLUME /tmp
#ARG SPRINGBOOT_APP=build/libs/KafkaSink-0.1.0-SNAPSHOT.jar
#COPY ${SPRINGBOOT_APP} app.jar
COPY build/libs/swir-java-client-0.0.1-SNAPSHOT.jar /app.jar
EXPOSE 8090
ENTRYPOINT ["java","-Djava.security.egd=file:/dev/./urandom","-jar","/app.jar"]