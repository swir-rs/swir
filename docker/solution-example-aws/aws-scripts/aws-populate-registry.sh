repoUri="$1.dkr.ecr.$2.amazonaws.com"
echo "${repoUri}"
aws ecr get-login-password --region $2 | docker -v login --username AWS --password-stdin "${repoUri}"

aws ecr create-repository --repository-name swir-example-java-source
docker tag swir-example-java-source:v2 ${repoUri}/swir-example-java-source:v2
docker push ${repoUri}/swir-example-java-source

aws ecr create-repository --repository-name swir-example-java-processor
docker tag swir-example-java-processor:v2 ${repoUri}/swir-example-java-processor:v2
docker push ${repoUri}/swir-example-java-processor

aws ecr create-repository --repository-name swir-example-python-sink
docker tag swir-example-python-sink:v2 ${repoUri}/swir-example-python-sink:v2
docker push ${repoUri}/swir-example-python-sink

aws ecr create-repository --repository-name swir-example-python-processor
docker tag swir-example-python-processor:v2 ${repoUri}/swir-example-python-processor:v2
docker push ${repoUri}/swir-example-python-processor

aws ecr create-repository --repository-name swir
docker tag swir:v2 ${repoUri}/swir:v2
docker push ${repoUri}/swir

aws ecr create-repository --repository-name swir-aws-example-configurator
docker tag swir-aws-example-configurator:v2 ${repoUri}/swir-aws-example-configurator:v2
docker push ${repoUri}/swir-aws-example-configurator


#docker -v logout "${repoUri}"
#aws ecr delete-repository --repository-name $1

