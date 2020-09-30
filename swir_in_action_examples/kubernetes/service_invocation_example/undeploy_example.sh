#!/bin/bash
kubectl -n $1 delete deployment helpdesk
kubectl -n $1 delete deployment magazines
kubectl -n $1 delete deployment books

