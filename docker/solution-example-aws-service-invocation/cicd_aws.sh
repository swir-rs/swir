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
./cicd.sh
cd docker/solution-example-aws-service-invocation

source ../../secure.sh

cd ../solution-example-service-invocation/swir-configurator
printf "\n**********************\n"
printf "\nConfigurator \n"
docker build --tag swir-example-aws-si-configurator:v3 .
printf "\nConfigurator... done"
printf "\n**********************\n"


cd ../../solution-example-service-invocation/swir-python-http-server
printf "\n**********************\n"
printf "\nPython HTTP server  \n"

./build.sh
docker build --tag swir-example-si-python-http-server:v3 . 

printf "\nPython HTTP server  done"
printf "\n**********************\n"


cd ../swir-python-grpc-client
printf "\n**********************\n"
printf "\nPython GRPC client  \n"

./build.sh
docker build --tag swir-example-si-python-grpc-client:v3 .

printf "\nPython GRPC client ... done \n"
printf "\n**********************\n"


cd ../../solution-example-aws-service-invocation/
cd aws-scripts
./aws-create-table.sh "swir-service-discovery"
./aws-populate-registry.sh $1 $2
./aws-create-role.sh 
python3 aws-log-groups.py CREATE
python3 aws-create-deployment.py $1 $2

cd ..
