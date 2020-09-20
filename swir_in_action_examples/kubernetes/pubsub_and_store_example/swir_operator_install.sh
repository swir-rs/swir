#!/bin/bash
kubectl create ns $1
kubectl -n $1 delete configmap swir-operator-config
kubectl -n $1 create configmap swir-operator-config --from-file=./processor_1.yaml --from-file=./processor_2.yaml --from-file=./processor_3.yaml --from-file=./source.yaml --from-file=./sink.yaml


kubectl -n $1 apply -f service_account.yaml
kubectl -n $1 apply -f role.yaml



MYVARVALUE="$1"
template=$(cat "role_binding.yaml" | sed "s/namespace: \"swir-ns\"/namespace: $MYVARVALUE/g")
echo "$template" | kubectl apply -n $1 -f -




MYVARVALUE="deployments\/$2"
template=$(cat "operator.yaml" | sed "s/deployments\/swir/$MYVARVALUE/g")
echo "$template" | kubectl apply -n $1 -f -


