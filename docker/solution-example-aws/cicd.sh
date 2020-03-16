
### Put your AWS credentials in ../../secure.sh
if [ ! -f ../../secure.sh ]; then
    echo "Put your AWS credentials in ../../secure.sh   !!!!"
    exit 1
fi

./aws-delete-table.sh "swir-locks"
./aws-create-table.sh "swir-locks"
./aws-kinesis-create-stream.sh aws_processor_orders_blue 2
./aws-kinesis-create-stream.sh aws_processor_inventory_green 2
./aws-kinesis-create-stream.sh aws_processor_billing_blue 2
./aws-kinesis-create-stream.sh aws_sink_green 2


# Compile, build and generate necessary docker images


cd ./swir-configurator
docker build --tag swir-example-configurator:v2 .

cd ../../solution-example/swir-python-processor
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

cd ../../solution-example-aws

docker network create docker_swir-net-aws	

docker-compose -f docker-compose-example-sidecars.yaml -p app down
docker-compose -f docker-compose-example-applications.yaml -p app down

source ../../secure.sh

docker-compose -f docker-compose-example-sidecars.yaml -p app up -d
docker-compose -f docker-compose-example-applications.yaml -p app up -d


#Sidecar logs 
#docker-compose  -f docker-compose-example-sidecars.yaml -p app logs -ft

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
./cleanup.sh


