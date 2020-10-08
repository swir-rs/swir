#!/bin/bash
eval $(minikube docker-env) # this has to be executed in a shell that will build the docker
cwd=$(pwd)
cd ../../../
root_dir=$(pwd)
docker pull swir/swir:v0.3.1
cd $cwd
cd ../../docker/pubsub_and_store_example/swir-python-processor
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
cd $cwd
