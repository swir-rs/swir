#!/bin/bash
if ! [ -n "$1" ]; then
    echo "Set accound id for your AWS subscription";
    exit 1
fi

if ! [ -n "$2" ]; then
    echo "Set valid AWS region "
    exit 1
fi

echo "Accout " $1 " region " $2
cwd=$(pwd)
cd ../../../
docker pull swir/swir:v0.3.2
root_folder=$(pwd)
image_folder=$root_folder/swir_in_action_examples/docker/service_invocation_example
cd $cwd
source $root_folder/secure.sh

cd $image_folder/swir-configurator
printf "\n**********************\n"
printf "\nConfigurator \n"
docker build --tag swir-example-aws-si-configurator:v0.3.2 .
printf "\nConfigurator... done"
printf "\n**********************\n"


cd $image_folder/swir-python-http-server
printf "\n**********************\n"
printf "\nPython HTTP server  \n"

docker build --tag swir-example-si-python-http-server:v0.3.2 . 

printf "\nPython HTTP server  done"
printf "\n**********************\n"


cd ../swir-python-grpc-client
printf "\n**********************\n"
printf "\nPython GRPC client  \n"
cp $root_folder/grpc_api/*.proto .
docker build --tag swir-example-si-python-grpc-client:v0.3.2 .
rm *.proto
printf "\nPython GRPC client ... done \n"
printf "\n**********************\n"


cd $cwd
cd aws-scripts
./aws-create-table.sh "swir-service-discovery"
./aws-populate-registry.sh $1 $2
./aws-create-role.sh 
python3 aws-log-groups.py CREATE
python3 aws-create-deployment.py $1 $2

cd $cwd
