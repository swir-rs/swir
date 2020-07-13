aws ecr delete-repository --repository-name swir-example-java-source --force
aws ecr delete-repository --repository-name swir-example-java-processor --force
aws ecr delete-repository --repository-name swir-example-python-sink --force
aws ecr delete-repository --repository-name swir-example-python-processor --force
aws ecr delete-repository --repository-name swir --force
aws ecr delete-repository --repository-name swir-aws-example-configurator --force

docker rmi ${repoUri}/swir-example-java-source:v3
docker rmi ${repoUri}/swir-example-java-processor:v3
docker rmi ${repoUri}/swir-example-python-sink:v3
docker rmi ${repoUri}/swir-example-python-processor:v3
docker rmi ${repoUri}/swir:v3
docker rmi ${repoUri}/swir-aws-example-configurator:v3

