#!/bin/bash

kubectl -n $1 delete deployment swir-operator
kubectl -n $1 delete clusterrole swir-operator
kubectl -n $1 delete clusterrolebinding swir-operator
kubectl -n $1 delete serviceaccount swir-operator
kubectl -n $1 delete configmap swir-operator-config
kubectl delete ns $1
