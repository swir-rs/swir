
### Put your AWS credentials in ../../secure.sh
if [ ! -f ../../secure.sh ]; then
    echo "Put your AWS credentials in ../../secure.sh   !!!!"
    exit 1
fi

cd ../../
./cicd.sh
cd docker/solution-example-aws

source ../../secure.sh

cd aws-scripts
./aws-create-streams.sh
cd ..

cd ./swir-configurator
printf "\n**********************\n"
printf "\nConfigurator \n"
# Compile, build and generate necessary docker images
docker build --tag swir-aws-example-configurator:v3 .

printf "\nConfigurator... done"
printf "\n**********************\n"

cd ../../solution-example/swir-python-processor

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
printf "\nPython GRPC sink... done  \n"
printf "\n**********************\n"

cd ../../solution-example-aws

docker network create docker_swir-net-aws	

docker-compose -f docker-compose-example-sidecars.yaml -p app down
docker-compose -f docker-compose-example-applications.yaml -p app down


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
./cicd_standalone_cleanup.sh

