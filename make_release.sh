#!/bin/bash
old_version=$1
new_version=$2
echo "Version $1 -> $2"
find . -name "*.sh" -exec grep -Hn ":$old_version" '{}' \; -exec sed -i "s/:$old_version/:$new_version/g" {} \;
find . -name "*.yaml" -exec grep -Hn ":$old_version" '{}' \; -exec sed -i "s/:$old_version/:$new_version/g" {} \;

root_dir=$(pwd)

./build.sh swir.yaml $new_version
cd ../swir-demo-clients
./build.sh $2 $root_dir
cd $root_dir

cd ../swir-operator
./make_release.sh $1 $2

cd $root_dir
docker push swir/swir:$new_version
docker push swir/swir-example-pubsub-configurator:$new_version
docker push swir/swir-example-pubsub-python-processor:$new_version
docker push swir/swir-example-pubsub-java-processor:$new_version
docker push swir/swir-example-pubsub-java-source:$new_version
docker push swir/swir-example-pubsub-python-sink:$new_version
docker push swir/swir-example-si-configurator:$new_version
docker push swir/swir-example-si-python-http-server:$new_version
docker push swir/swir-example-si-python-grpc-client:$new_version
docker push swir/swir-operator:${new_version}
#docker push swir/swir-example-aws-pubsub-configurator:$new_version
#docker push swir/swir-example-aws-si-configurator:$new_version


