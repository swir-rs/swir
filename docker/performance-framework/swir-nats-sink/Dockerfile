FROM gradle:jdk14
COPY build.gradle ./
COPY settings.gradle ./
COPY src ./src
RUN gradle --no-daemon  bootJar
ENTRYPOINT ["java","-Djava.security.egd=file:/dev/./urandom","-jar","build/libs/swir-nats-sink-0.0.1-SNAPSHOT.jar"]
