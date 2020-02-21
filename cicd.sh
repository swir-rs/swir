#necessary certs for HTTPs but only on the first run
#./generate-cert.sh

cargo build --release --features="with_nats"
docker build . --build-arg executable=target/release/swir --build-arg client=docker/performance-framework/swir-java-client/build/libs/swir-java-client-0.0.1-SNAPSHOT.jar --build-arg swir_config=swir_docker.yaml -t swir:v2

#cargo build --features="with_nats"
#docker build . --build-arg executable=target/debug/swir --build-arg client=clients/swir-java-client/build/libs/swir-java-client-0.0.1-SNAPSHOT.jar --build-arg swir_config=swir_docker.yaml -t swir:v2

docker-compose -f docker/docker-compose-infr.yml down --remove-orphans

# this should deploy the infrastructure
# Docker instance names/network name created by docker compose could change
docker-compose -f docker/docker-compose-infr.yml up -d

# cd docker/solution-example
# ./cicd.sh
# cd ../../

# docker-compose -f docker/solution-example/docker-compose-example-applications.yaml -p app logs -t -f 
# docker-compose -f docker/solution-example/docker-compose-example-sidecars.yaml -p app down --remove-orphans

cd docker/performance-framework
./cicd.sh
cd ../../
#docker-compose -p pf -f docker/performance-framework/docker-compose-swir.yml down --remove-orphans
