aws ecr delete-repository --repository-name swir-example-si-python-http-server --force
aws ecr delete-repository --repository-name swir-example-si-python-grpc-client --force
aws ecr delete-repository --repository-name swir/swir --force
aws ecr delete-repository --repository-name swir-example-aws-si-configurator --force

docker rmi ${repoUri}/swir-example-si-python-http-server:v0.3.1
docker rmi ${repoUri}/swir-example-si-python-grpc-client:v0.3.1
docker rmi ${repoUri}/swir/swir:v0.3.1
docker rmi ${repoUri}/swir-example-aws-si-configurator:v0.3.1

