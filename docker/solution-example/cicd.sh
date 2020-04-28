# Compile, build and generate necessary docker images
cd ../../
./cicd.sh
cd docker/solution-example


cd ./swir-configurator
printf "\n**********************\n"
printf "\nConfigurator \n"

docker build --tag swir-example-configurator:v3 .

printf "\nConfigurator... done"
printf "\n**********************\n"


cd ../swir-python-processor
printf "\n**********************\n"
printf "\nPython processor  \n"

./build.sh
docker build --tag swir-example-python-processor:v3 .

printf "\nPython processor  done"
printf "\n**********************\n"


cd ../swir-java-processor
printf "\n**********************\n"
printf "\nJava processor  \n"

./gradlew clean bootJar
docker build --tag swir-example-java-processor:v3 .

printf "\nJava processor  done"
printf "\n**********************\n"


cd ../swir-java-source
printf "\n**********************\n"
printf "\nJava GRPC source  \n"

./gradlew clean build installDist assembleDist
docker build --tag swir-example-java-source:v3 .

printf "\nJava GRPC source...done"
printf "\n**********************\n"


cd ../swir-python-sink

printf "\n**********************\n"
printf "\nPython GRPC sink  \n"

./build.sh
docker build --tag swir-example-python-sink:v3 .

printf "Python GRPC sink... done  \n"
printf "\n**********************\n"


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
sleep 2
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
./cicd_cleanup.sh



