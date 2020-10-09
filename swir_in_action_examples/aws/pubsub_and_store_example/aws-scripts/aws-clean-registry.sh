aws ecr delete-repository --repository-name swir-example-java-source --force
aws ecr delete-repository --repository-name swir-example-java-processor --force
aws ecr delete-repository --repository-name swir-example-python-sink --force
aws ecr delete-repository --repository-name swir-example-python-processor --force
aws ecr delete-repository --repository-name swir/swir --force
aws ecr delete-repository --repository-name swir-aws-example-configurator --force

docker rmi ${repoUri}/swir-example-java-source:v0.3.2
docker rmi ${repoUri}/swir-example-java-processor:v0.3.2
docker rmi ${repoUri}/swir-example-python-sink:v0.3.2
docker rmi ${repoUri}/swir-example-python-processor:v0.3.2
docker rmi ${repoUri}/swir/swir:v0.3.2
docker rmi ${repoUri}/swir-aws-example-configurator:v0.3.2

