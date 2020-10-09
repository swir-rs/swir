# Compile, build and generate necessary docker images
#!/bin/bash
cwd=$(pwd)
cd ../../../
root_dir=$(pwd)
docker pull swir/swir:v0.3.1
cd $cwd

cd ./swir-configurator
printf "\n**********************\n"
printf "\nConfigurator \n"

docker build --tag swir-example-configurator:v0.3.1 . 

printf "\nConfigurator... done"
printf "\n**********************\n"


cd ../swir-python-processor
printf "\n**********************\n"
printf "\nPython processor  \n"
cp $root_dir/grpc_api/*.proto .
docker build --tag swir-example-python-processor:v0.3.1 .
rm *.proto
printf "\nPython processor  done"
printf "\n**********************\n"


cd ../swir-java-processor
printf "\n**********************\n"
printf "\nJava processor  \n"
docker build --tag swir-example-java-processor:v0.3.1 .
printf "\nJava processor  done"
printf "\n**********************\n"


cd ../swir-java-source
printf "\n**********************\n"
printf "\nJava GRPC source  \n"
cp -r $root_dir/grpc_api/ .
docker build --tag swir-example-java-source:v0.3.1 .
rm -rf grpc_api
printf "\nJava GRPC source...done"
printf "\n**********************\n"


cd ../swir-python-sink

printf "\n**********************\n"
printf "\nPython GRPC sink  \n"
cp $root_dir/grpc_api/*.proto .
docker build --tag swir-example-python-sink:v0.3.1 .
rm *.proto
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

printf "\n**********************\n"
printf "\nTracing logs can be found \n\n"
printf "\nhttp://127.0.0.1:16686/search\n\n"
printf "\n**********************\n"

sleep 5



#Sidecar logs 
docker-compose  -f docker-compose-example-sidecars.yaml -p app logs -ft

#docker logs app_order-processor-sidecar_1
#docker logs app_shipments-sink-sidecar_1
#docker logs app_billing-processor-sidecar_1
#docker logs app_inventory-processor-sidecar_1
#docker logs app_order-generator-sidecar_1

#Application logs
docker-compose  -f docker-compose-example-applications.yaml -p app logs -ft

#docker logs app_order-generator_1
#docker logs app_order-processor_1
#docker logs app_inventory-processor_1
#docker logs app_billing-processor_1
#docker logs app_shipments-sink_1




#clean all
#./cleanup_example.sh



