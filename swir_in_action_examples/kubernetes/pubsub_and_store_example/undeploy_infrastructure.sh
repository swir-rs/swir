#!/bin/bash
helm delete kafka --namespace kafka
helm delete redis --namespace redis
helm delete nats --namespace nats

kubectl delete namespace kafka
kubectl delete namespace redis
kubectl delete namespace nats

./swir_operator_remove.sh $2
../jaeger_uninstall.sh $1
kubectl delete namespace $1

