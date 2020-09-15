#!/bin/bash
echo $1
kubectl -n $1 delete configmap swir-operator-config
kubectl -n $1 create configmap swir-operator-config --from-file=pubsub_and_store_example/processor_1.yaml --from-file=pubsub_and_store_example/processor_2.yaml --from-file=pubsub_and_store_example/processor_3.yaml --from-file=pubsub_and_store_example/source.yaml --from-file=pubsub_and_store_example/sink.yaml


kubectl -n $1 apply -f https://raw.githubusercontent.com/swir-rs/swir-operator/master/deploy/service_account.yaml
kubectl -n $1 apply -f https://raw.githubusercontent.com/swir-rs/swir-operator/master/deploy/role.yaml
kubectl -n $1 apply -f https://raw.githubusercontent.com/swir-rs/swir-operator/master/deploy/role_binding.yaml
kubectl -n $1 apply -f https://raw.githubusercontent.com/swir-rs/swir-operator/master/deploy/operator.yaml

#kubectl -n $1 apply -f /home/dawid/Workspace/swir-operator/deploy/operator.yaml
