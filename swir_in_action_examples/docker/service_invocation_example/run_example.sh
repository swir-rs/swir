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

docker build --tag swir-example-si-configurator:v0.3.1 .

printf "\nConfigurator... done"
printf "\n**********************\n"


cd ../swir-python-http-server
printf "\n**********************\n"
printf "\nPython HTTP server  \n"

docker build --tag swir-example-si-python-http-server:v0.3.1 . 

printf "\nPython HTTP server  done"
printf "\n**********************\n"


cd ../swir-python-grpc-client
printf "\n**********************\n"
printf "\nPython GRPC client  \n"
cp $root_dir/grpc_api/*.proto .
docker build --tag swir-example-si-python-grpc-client:v0.3.1 .
rm *.proto
printf "\nPython GRPC client ... done \n"
printf "\n**********************\n"


cd ..

docker-compose -f docker-compose-si-example-sidecars.yaml -p app down
docker-compose -f docker-compose-si-example-applications.yaml -p app down

# Create necessary topics for Kafka

sleep 5

docker-compose -f docker-compose-si-example-sidecars.yaml -p app up -d
docker-compose -f docker-compose-si-example-applications.yaml -p app up -d


printf "\n**********************\n"
printf "\nTracing logs can be found \n\n"
printf "\nhttp://127.0.0.1:16686/search\n\n"
printf "\n**********************\n"

sleep 5

#Sidecar logs 
docker-compose  -f docker-compose-si-example-sidecars.yaml -p app logs -ft

#Application logs
docker-compose  -f docker-compose-si-example-applications.yaml -p app logs -ft

#clean all
./cleanup_example.sh



