FROM openjdk:14
COPY build/libs/swir-java-client-0.0.1-SNAPSHOT.jar /app.jar
EXPOSE 8090
ENTRYPOINT ["java","-Djava.security.egd=file:/dev/./urandom","-jar","/app.jar"]