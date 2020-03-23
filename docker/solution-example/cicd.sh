# Compile, build and generate necessary docker images
../../cicd.sh

cd ./swir-configurator
docker build --tag swir-example-configurator:v2 .

cd ../swir-python-processor
./build.sh
docker build --tag swir-example-python-processor:v2 .

cd ../swir-java-processor
./gradlew clean bootJar
docker build --tag swir-example-java-processor:v2 .

cd ../swir-java-source
./gradlew clean build installDist assembleDist
docker build --tag swir-example-java-source:v2 .

cd ../swir-python-sink
./build.sh
docker build --tag swir-example-python-sink:v2 .

cd ..

docker-compose -f docker-compose-example-sidecars.yaml -p app down
docker-compose -f docker-compose-example-applications.yaml -p app down
docker-compose -f docker-compose-infr.yml -p docker down --remove-orphans
docker-compose -f docker-compose-infr.yml -p docker up -d

# Create necessary topics for Kafka

sleep 5

docker exec -t docker_kafka_1 kafka-topics.sh --bootstrap-server :9094 --create --topic processor1_kafka_blue --partitions 2 --replication-factor 1
docker exec -t docker_kafka_1 kafka-topics.sh --bootstrap-server :9094 --create --topic processor3_kafka_red --partitions 2 --replication-factor 1
docker exec -t docker_kafka_1 kafka-topics.sh --bootstrap-server :9094 --create --topic sink1_kafka_green --partitions 2 --replication-factor 1

docker-compose -f docker-compose-example-sidecars.yaml -p app up -d
docker-compose -f docker-compose-example-applications.yaml -p app up -d


#Sidecar logs 
docker-compose  -f docker-compose-example-sidecars.yaml -p app logs -ft

#docker logs solution-example_order-processor-sidecar_1
#docker logs solution-example_shipments-sink-sidecar_1
#docker logs solution-example_billing-processor-sidecar_1
#docker logs solution-example_inventory-processor-sidecar_1
#docker logs solution-example_order-generator-sidecar_1

#Application logs
docker-compose  -f docker-compose-example-applications.yaml -p app logs -ft

#docker logs solution-example_order-generator_1
#docker logs solution-example_order-processor_1
#docker logs solution-example_inventory-processor_1
#docker logs solution-example_billing-processor_1
#docker logs solution-example_shipments-sink_1


#clean all
docker-compose -p app -f docker-compose-example-applications.yaml down --remove-orphans
docker-compose -p app -f docker-compose-example-sidecars.yaml down --remove-orphans
docker-compose -f docker-compose-infr.yml -p docker down --remove-orphans


