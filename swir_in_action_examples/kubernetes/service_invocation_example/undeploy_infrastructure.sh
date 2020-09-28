#!/bin/bash
./swir_operator_remove.sh $2
../jaeger_uninstall.sh $1
kubectl delete namespace $1

