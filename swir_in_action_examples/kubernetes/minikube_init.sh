#!/bin/bash
#minikube start --vm-driver=virtualbox --cpus='8' --memory='16384mb' --disk-size='20000mb'
#minikube start  --cpus='8' --memory='16384mb' --disk-size='20000mb'
minikube start 
minikube addons enable ingress
kubectl create ns swir



