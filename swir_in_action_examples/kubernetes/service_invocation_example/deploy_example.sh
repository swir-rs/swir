#!/bin/bash
kubectl -n swir create configmap books-config --from-file=./books.yaml 
kubectl -n swir create configmap magazines-config --from-file=./magazines.yaml 
kubectl -n swir create configmap helpdesk-config --from-file=./helpdesk.yaml
cert_location="../../docker/service_invocation_example/swir-configurator/certificates"
kubectl -n swir create configmap certs-config --from-file=$cert_location/ca-chain.cert.pem --from-file=$cert_location/client.internal_grpc.swir.rs.cert.pem --from-file=$cert_location/client.internal_grpc.swir.rs.key.pem --from-file=$cert_location/server.internal_grpc.swir.rs.cert.pem --from-file=$cert_location/server.internal_grpc.swir.rs.key.pem



kubectl -n swir apply -f ./01_books.yaml
kubectl -n swir apply -f ./02_magazines.yaml
kubectl -n swir apply -f ./03_helpdesk.yaml
