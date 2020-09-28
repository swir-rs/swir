#!/bin/bash
kubectl create ns $1
kubectl -n $1 delete configmap swir-operator-config
kubectl -n $1 create configmap swir-operator-config --from-file=./books.yaml --from-file=./helpdesk.yaml --from-file=./magazines.yaml 


cert_location="../../docker/service_invocation_example/swir-configurator/certificates"
kubectl -n $1 delete configmap swir-operator-certs
kubectl -n $1 create configmap swir-operator-certs --from-file=$cert_location/ca-chain.cert.pem --from-file=$cert_location/client.internal_grpc.swir.rs.cert.pem --from-file=$cert_location/client.internal_grpc.swir.rs.key.pem --from-file=$cert_location/server.internal_grpc.swir.rs.cert.pem --from-file=$cert_location/server.internal_grpc.swir.rs.key.pem


kubectl -n $1 apply -f service_account.yaml
kubectl -n $1 apply -f role.yaml

MYVARVALUE="$1"
template=$(cat "role_binding.yaml" | sed "s/namespace: \"swir-ns\"/namespace: $MYVARVALUE/g")
echo "$template" | kubectl apply -n $1 -f -



MYVARVALUE1="configs\/$2\/"
MYVARVALUE2="certs\/$2\/"
template=$(cat "operator.yaml" | sed "s/configs\/namespace/$MYVARVALUE1/g" | sed "s/certs\/namespace/$MYVARVALUE2/g")
echo "$template" | kubectl apply -n $1 -f -


