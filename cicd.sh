
docker rmi --no-prune swir:v3
docker build -f swir_in_action_examples/docker/Dockerfile_build_stage1 -t swir_builder:latest .
if [ -z "$1" ]
then
    docker build --build-arg swir_config=swir.yaml -t swir:v3 -f swir_in_action_examples/docker/Dockerfile_build_stage2 .
else
    docker build --build-arg swir_config=$1 -t swir:v3 -f swir_in_action_examples/docker/Dockerfile_build_stage2 .    
fi    
