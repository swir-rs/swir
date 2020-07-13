FROM gradle:jdk14
COPY build.gradle ./
COPY settings.gradle ./
COPY src ./src
RUN gradle --no-daemon bootJar
EXPOSE 8090
ENTRYPOINT ["java","-Djava.security.egd=file:/dev/./urandom","-jar","build/libs/swir-java-client-0.0.1-SNAPSHOT.jar"]
