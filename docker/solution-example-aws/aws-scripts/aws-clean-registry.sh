aws ecr delete-repository --repository-name swir-example-java-source --force
aws ecr delete-repository --repository-name swir-example-java-processor --force
aws ecr delete-repository --repository-name swir-example-python-sink --force
aws ecr delete-repository --repository-name swir-example-python-processor --force
aws ecr delete-repository --repository-name swir --force
aws ecr delete-repository --repository-name swir-aws-example-configurator --force

docker rmi ${repoUri}/swir-example-java-source:v2
docker rmi ${repoUri}/swir-example-java-processor:v2
docker rmi ${repoUri}/swir-example-python-sink:v2
docker rmi ${repoUri}/swir-example-python-processor:v2
docker rmi ${repoUri}/swir:v2
docker rmi ${repoUri}/swir-aws-example-configurator:v2

