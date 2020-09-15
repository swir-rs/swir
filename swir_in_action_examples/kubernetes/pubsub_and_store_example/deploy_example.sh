#!/bin/bash
# kubectl -n swir create configmap source-config --from-file=./source.yaml 
# kubectl -n swir create configmap processor1-config --from-file=./processor_1.yaml 
# kubectl -n swir create configmap processor2-config --from-file=./processor_2.yaml
# kubectl -n swir create configmap processor3-config --from-file=./processor_3.yaml
# kubectl -n swir create configmap sink-config --from-file=./sink.yaml


kubectl -n swir apply -f ./05_sink.yaml
kubectl -n swir apply -f ./04_processor3.yaml
kubectl -n swir apply -f ./03_processor2.yaml
kubectl -n swir apply -f ./02_processor1.yaml
kubectl -n swir apply -f ./01_source.yaml
