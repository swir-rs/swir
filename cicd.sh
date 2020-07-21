#!/bin/bash
echo "**************************"
echo "Building SWIR sidecar image"
echo ""
echo "This is slow and takes time on the first build"
echo ""
echo "**************************"
docker rmi --no-prune swir/swir:v3
docker build -f build/Dockerfile_build_stage1 -t swir_builder:latest .
if [ -z "$1" ]
then
    docker build --build-arg swir_config=swir.yaml -t swir/swir:v3 -f build/Dockerfile_build_stage2 .
else
    docker build --build-arg swir_config=$1 -t swir/swir:v3 -f build/Dockerfile_build_stage2 .    
fi
echo "**************************"
