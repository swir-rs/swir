#!/bin/bash
./check_env.sh
./minikube_init.sh
./jaeger_install.sh 
./swir_operator_install.sh swir-ns


