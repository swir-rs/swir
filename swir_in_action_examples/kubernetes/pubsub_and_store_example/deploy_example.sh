#!/bin/bash

kubectl -n $1 apply -f ./05_sink.yaml
kubectl -n $1 apply -f ./04_processor3.yaml
kubectl -n $1 apply -f ./03_processor2.yaml
kubectl -n $1 apply -f ./02_processor1.yaml
kubectl -n $1 apply -f ./01_source.yaml
