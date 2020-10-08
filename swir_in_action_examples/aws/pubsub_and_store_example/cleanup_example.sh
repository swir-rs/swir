#!/bin/bash
if ! [ -n "$1" ]; then
    echo "Set postfix for the cluster you want to delete as printed during ./run_example.sh"
    exit 1
fi

cd aws-scripts
./aws-delete-streams.sh
python3 aws-delete-deployment.py $1

python3 aws-log-groups.py DELETE
./aws-clean-registry.sh
./aws-delete-role.sh
cd ..

docker rmi -f $(docker image list 164450575887.dkr.ecr.eu-west-1.amazonaws.com/*:v0.3.1 -q)
