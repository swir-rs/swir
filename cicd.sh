
docker rmi swir:v3
docker build -f docker/Dockerfile_build_stage1 -t swir_builder:latest .
if [ -z "$1" ]
then
    docker build --build-arg swir_config=swir.yaml -t swir:v3 -f docker/Dockerfile_build_stage2 .
else
    docker build --build-arg swir_config=$1 -t swir:v3 -f docker/Dockerfile_build_stage2 .    
fi    
