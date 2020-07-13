FROM maven:3.6-slim
COPY pom.xml ./
COPY src ./src
RUN mvn package 
EXPOSE 8090
ENTRYPOINT ["java","-Djava.security.egd=file:/dev/./urandom","-jar","target/swir-java-processor-0.0.1-SNAPSHOT.jar"]