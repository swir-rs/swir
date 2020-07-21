#!/bin/bash
kubectl -n swir delete deployment helpdesk
kubectl -n swir delete deployment magazines
kubectl -n swir delete deployment books


kubectl -n swir delete configmap certs-config
kubectl -n swir delete configmap books-config
kubectl -n swir delete configmap magazines-config 
kubectl -n swir delete configmap helpdesk-config 
