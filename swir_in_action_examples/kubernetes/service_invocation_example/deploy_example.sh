#!/bin/bash

kubectl -n $1 apply -f ./01_books.yaml
kubectl -n $1 apply -f ./02_magazines.yaml
kubectl -n $1 apply -f ./03_helpdesk.yaml
