#!/bin/bash
#minikube start --vm-driver=virtualbox --cpus='8' --memory='16384mb' --disk-size='20000mb'
#minikube start  --cpus='8' --memory='16384mb' --disk-size='20000mb'
minikube start --cpus='4' --memory='4096mb'
minikube addons enable ingress




