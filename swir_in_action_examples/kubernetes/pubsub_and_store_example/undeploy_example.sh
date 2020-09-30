#!/bin/bash
kubectl -n $1 delete deployment source
kubectl -n $1 delete deployment processor1
kubectl -n $1 delete deployment processor2
kubectl -n $1 delete deployment processor3
kubectl -n $1 delete deployment sink

