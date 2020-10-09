repoUri="$1.dkr.ecr.$2.amazonaws.com"
echo "${repoUri}"
aws ecr get-login-password --region $2 | docker -v login --username AWS --password-stdin "${repoUri}"

aws ecr create-repository --repository-name swir-example-aws-si-configurator
docker tag swir-example-aws-si-configurator:v0.3.2 ${repoUri}/swir-example-aws-si-configurator:v0.3.2
docker push ${repoUri}/swir-example-aws-si-configurator

aws ecr create-repository --repository-name swir-example-si-python-http-server
docker tag swir-example-si-python-http-server:v0.3.2 ${repoUri}/swir-example-si-python-http-server:v0.3.2
docker push ${repoUri}/swir-example-si-python-http-server

aws ecr create-repository --repository-name swir-example-si-python-grpc-client
docker tag swir-example-si-python-grpc-client:v0.3.2 ${repoUri}/swir-example-si-python-grpc-client:v0.3.2
docker push ${repoUri}/swir-example-si-python-grpc-client

aws ecr create-repository --repository-name swir/swir
docker tag swir:v0.3.2 ${repoUri}/swir/swir:v0.3.2
docker push ${repoUri}/swir/swir


#docker -v logout "${repoUri}"
#aws ecr delete-repository --repository-name $1

