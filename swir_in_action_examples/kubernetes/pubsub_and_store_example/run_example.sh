#!/bin/bash
if ! [ -n "$1" ]; then
    echo "Provide namespace for deployment"
    exit 1
fi

if ! [ -n "$2" ]; then
    echo "Provide namespace for operator to use"
    exit 1
fi

echo "Deployment namespace  " $1 " operator namespace " $2
deployment_name=$1
operator_namespace=$2
cwd=$(pwd)
cd ..
./setup.sh $deployment_name
cd $cwd
./swir_operator_install.sh $operator_namespace $deployment_name 
./deploy_infrastructure.sh 
./build_clients.sh
./deploy_example.sh $deployment_name

printf "\n\n************************************\n\n"
printf "\n\nTo check logs run: \n\n "
printf "\n./display_source_logs.sh %s \n" $deployment_name
printf "\n./display_sink_logs.sh %s \n" $deployment_name
printf "\n\n************************************\n\n"

../tracing_dashboard.sh $deployment_name


