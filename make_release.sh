#!/bin/bash
old_version=$1
new_version=$2
echo "Version $1 -> $2"
find . -name "*.sh" -exec grep -Hn ":$old_version" '{}' \; -exec sed -i "s/:$old_version/:$new_version/g" {} \;
find . -name "*.yaml" -exec grep -Hn ":$old_version" '{}' \; -exec sed -i "s/:$old_version/:$new_version/g" {} \;

root_dir=$(pwd)

./build.sh swir.yaml $new_version

cd swir_in_action_examples/docker/pubsub_and_store_example

cd ./swir-configurator
printf "\n**********************\n"
printf "\nConfigurator \n"
docker build --tag swir/swir-example-pubsub-configurator:$new_version .
printf "\nConfigurator... done"
printf "\n**********************\n"
cd ../swir-python-processor
printf "\n**********************\n"
printf "\nPython processor  \n"
cp $root_dir/grpc_api/*.proto .
docker build --tag swir/swir-example-pubsub-python-processor:$new_version .
rm *.proto
printf "\nPython processor  done"
printf "\n**********************\n"
cd ../swir-java-processor
printf "\n**********************\n"
printf "\nJava processor  \n"
docker build --tag swir/swir-example-pubsub-java-processor:$new_version .
printf "\nJava processor  done"
printf "\n**********************\n"
cd ../swir-java-source
printf "\n**********************\n"
printf "\nJava GRPC source  \n"
cp -r $root_dir/grpc_api/ .
docker build --tag swir/swir-example-pubsub-java-source:$new_version .
rm -rf grpc_api
printf "\nJava GRPC source...done"
printf "\n**********************\n"
cd ../swir-python-sink
printf "\n**********************\n"
printf "\nPython GRPC sink  \n"
cp $root_dir/grpc_api/*.proto .
docker build --tag swir/swir-example-pubsub-python-sink:$new_version .
rm *.proto
printf "Python GRPC sink... done  \n"
printf "\n**********************\n"

cd $root_dir
cd swir_in_action_examples/docker/service_invocation_example
cd ./swir-configurator
printf "\n**********************\n"
printf "\nConfigurator \n"
docker build --tag swir/swir-example-si-configurator:$new_version .
docker build --tag swir/swir-example-si-aws-configurator:$new_version .

printf "\nConfigurator... done"
printf "\n**********************\n"


cd ../swir-python-http-server
printf "\n**********************\n"
printf "\nPython HTTP server  \n"
docker build --tag swir/swir-example-si-python-http-server:$new_version .

printf "\nPython HTTP server  done"
printf "\n**********************\n"


cd ../swir-python-grpc-client
printf "\n**********************\n"
printf "\nPython GRPC client  \n"
cp $root_dir/grpc_api/*.proto .
docker build --tag swir/swir-example-si-python-grpc-client:$new_version .
rm *.proto
printf "\nPython GRPC client ... done \n"
printf "\n**********************\n"


cd $root_dir
cd swir_in_action_examples/aws/pubsub_and_store_example/swir-configurator
printf "\n**********************\n"
printf "\nConfigurator \n"
docker build --tag swir/swir-example-pubsub-aws-configurator:$new_version .
printf "\nConfigurator... done"
printf "\n**********************\n"

cd $root_dir
docker push swir/swir-example-pubsub-configurator:$new_version
docker push swir/swir-example-pubsub-python-processor:$new_version
docker push swir/swir-example-pubsub-java-processor:$new_version
docker push swir/swir-example-pubsub-java-source:$new_version
docker push swir/swir-example-pubsub-python-sink:$new_version
docker push swir/swir-example-si-configurator:$new_version
docker push swir/swir-example-si-python-http-server:$new_version
docker push swir/swir-example-si-python-grpc-client:$new_version
#docker push swir/swir-example-aws-pubsub-configurator:$new_version
#docker push swir/swir-example-aws-si-configurator:$new_version


