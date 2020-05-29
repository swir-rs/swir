if ! [ -n "$1" ]; then
    echo "Set accound id for your AWS subscription";
    exit 1
fi

if ! [ -n "$2" ]; then
    echo "Set valid AWS region "
    exit 1
fi

echo "Accout " $1 " region " $2

cd ../../
./cicd.sh swir.yaml
cd docker/solution-example-aws

source ../../secure.sh

cd ./swir-configurator
printf "\n**********************\n"
printf "\nConfigurator \n"
docker build --tag swir-aws-example-configurator:v3 .
printf "\nConfigurator... done"
printf "\n**********************\n"

cd ../../solution-example/swir-python-processor
printf "\n**********************\n"
printf "\nPython processor  \n"
cp ../../../grpc_api/*.proto .
docker build --tag swir-example-python-processor:v3 .
rm *.proto
printf "\nPython processor  done"
printf "\n**********************\n"

cd ../swir-java-processor
printf "\n**********************\n"
printf "\nJava processor  \n"
docker build --tag swir-example-java-processor:v3 .
printf "\nJava processor  done"
printf "\n**********************\n"

cd ../swir-java-source
printf "\n**********************\n"
printf "\nJava GRPC source  \n"
cp -r ../../../grpc_api/ .
docker build --tag swir-example-java-source:v3 .
rm -rf grpc_api
printf "\nJava GRPC source...done"
printf "\n**********************\n"

cd ../swir-python-sink
printf "\n**********************\n"
printf "\nPython GRPC sink  \n"
cp ../../../grpc_api/*.proto .
docker build --tag swir-example-python-sink:v3 .
rm *.proto
printf "\nPython GRPC sink... done  \n"
printf "\n**********************\n"

cd ../../solution-example-aws/
cd aws-scripts
./aws-populate-registry.sh $1 $2
./aws-create-streams.sh
./aws-create-role.sh 
python3 aws-log-groups.py CREATE
python3 aws-create-deployment.py $1 $2

cd ..
