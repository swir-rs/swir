#!/bin/bash
if ! [ -n "$1" ]; then
    echo "Provide namespace for deployment"
    exit 1
fi

if ! [ -n "$2" ]; then
    echo "Provide namespace for operator to use"
    exit 1
fi


deployment_name=$1
operator_namespace=$2

./undeploy_example.sh $deployment_name
./undeploy_infrastructure.sh $deployment_name $operator_namespace
