# Compile, build and generate necessary docker images
cd ../../
./cicd.sh swir.yaml
cd docker/solution-example-service-invocation


cd ./swir-configurator
printf "\n**********************\n"
printf "\nConfigurator \n"

docker build --tag swir-example-si-configurator:v3 .

printf "\nConfigurator... done"
printf "\n**********************\n"


cd ../swir-python-http-server
printf "\n**********************\n"
printf "\nPython HTTP server  \n"

./build.sh
docker build --tag swir-example-si-python-http-server:v3 . 

printf "\nPython HTTP server  done"
printf "\n**********************\n"


cd ../swir-python-grpc-client
printf "\n**********************\n"
printf "\nPython GRPC client  \n"
cp ../../../grpc_api/*.proto .
docker build --tag swir-example-si-python-grpc-client:v3 .
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

#Sidecar logs 
docker-compose  -f docker-compose-si-example-sidecars.yaml -p app logs -ft

#Application logs
docker-compose  -f docker-compose-si-example-applications.yaml -p app logs -ft

#clean all
./cicd_cleanup.sh



