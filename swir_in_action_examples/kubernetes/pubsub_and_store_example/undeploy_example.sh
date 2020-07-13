#!/bin/bash
kubectl -n swir delete deployment source
kubectl -n swir delete deployment processor1
kubectl -n swir delete deployment processor2
kubectl -n swir delete deployment processor3
kubectl -n swir delete deployment sink


kubectl -n swir delete configmap source-config
kubectl -n swir delete configmap processor1-config 
kubectl -n swir delete configmap processor2-config 
kubectl -n swir delete configmap processor3-config 
kubectl -n swir delete configmap sink-config
