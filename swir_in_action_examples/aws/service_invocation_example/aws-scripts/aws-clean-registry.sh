aws ecr delete-repository --repository-name swir-example-si-python-http-server --force
aws ecr delete-repository --repository-name swir-example-si-python-grpc-client --force
aws ecr delete-repository --repository-name swir --force
aws ecr delete-repository --repository-name swir-example-aws-si-configurator --force

docker rmi ${repoUri}/swir-example-si-python-http-server:v3
docker rmi ${repoUri}/swir-example-si-python-grpc-client:v3
docker rmi ${repoUri}/swir/swir:v3
docker rmi ${repoUri}/swir-example-aws-si-configurator:v3

