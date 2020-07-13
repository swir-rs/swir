#!/bin/bash
helm delete kafka --namespace kafka
helm delete redis --namespace redis
helm delete nats --namespace nats


kubectl delete namespace kafka
kubectl delete namespace redis
kubectl delete namespace nats

