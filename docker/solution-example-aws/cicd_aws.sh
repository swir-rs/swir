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
cd docker/solution-example-aws

source ../../secure.sh

cd ./swir-configurator
docker build --tag swir-aws-example-configurator:v2 .

cd ../../solution-example/swir-python-processor
./build.sh
docker build --tag swir-example-python-processor:v2 .

cd ../swir-java-processor
./gradlew clean bootJar
docker build --tag swir-example-java-processor:v2 .

cd ../swir-java-source
./gradlew clean build installDist assembleDist
docker build --tag swir-example-java-source:v2 .

cd ../swir-python-sink
./build.sh
docker build --tag swir-example-python-sink:v2 .

cd ../../solution-example-aws/
cd aws-scripts
./aws-populate-registry.sh $1 $2
./aws-create-streams.sh
./aws-create-role.sh 
python3 aws-log-groups.py CREATE
python3 aws-create-deployment.py $1 $2

cd ..
