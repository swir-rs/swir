#!/bin/bash
eval $(minikube docker-env) # this has to be executed in a shell that will build the docker
cwd=$(pwd)
cd ../../../
rood_dir=$(pwd)
docker pull swir/swir:v0.3.1
cd $cwd
cd ../../docker/service_invocation_example/swir-python-http-server
printf "\n**********************\n"
printf "\nPython HTTP server  \n"

docker build --tag swir-example-si-python-http-server:v0.3.1 . 

printf "\nPython HTTP server  done"
printf "\n**********************\n"


cd ../swir-python-grpc-client
printf "\n**********************\n"
printf "\nPython GRPC client  \n"
cp $rood_dir/grpc_api/*.proto .
docker build --tag swir-example-si-python-grpc-client:v0.3.1 .
rm *.proto
printf "\nPython GRPC client ... done \n"
printf "\n**********************\n"


cd $cwd
