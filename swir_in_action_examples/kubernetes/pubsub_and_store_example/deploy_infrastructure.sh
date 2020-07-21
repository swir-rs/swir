#!/bin/bash

#helm init > /dev/null
helm repo add stable https://kubernetes-charts.storage.googleapis.com
helm repo add incubator https://kubernetes-charts-incubator.storage.googleapis.com
helm repo add bitnami https://charts.bitnami.com/bitnami

kubectl create ns kafka
kubectl create ns redis
kubectl create ns nats


helm install kafka --namespace kafka incubator/kafka > /dev/null
helm install redis --namespace redis --set usePassword=false bitnami/redis > /dev/null
helm install nats --namespace nats --set auth.enabled=false bitnami/nats > /dev/null
x=0
while [ $x -le 5 ]
do    
    x=$(kubectl get po -n kafka | grep "1/1" | wc -l)
    if [[ $x -le 5 ]]; then
	printf "Checking that Kafka cluster has been created... this could take a while\n"
	sleep 10
    fi
done


kubectl apply -f kafka-testclient.yaml
x=0
while [ $x -le 0 ]
do   
    x=$(kubectl get po -n kafka | grep "testclient" | grep "1/1" | wc -l)
    if [[ $x -le 0 ]]; then
	printf "Checking that Kafka test client is ready ... this could take a while\n"
	sleep 2
    fi
done

t=0
while [ $t -le 2 ]
do
    printf "Creating Kafka topics...\n"
    kubectl -n kafka exec testclient -- /opt/kafka/bin/kafka-topics.sh --zookeeper kafka-zookeeper:2181 --topic processor1_kafka_blue --create --partitions 1 --replication-factor 1 > /dev/null
    kubectl -n kafka exec testclient -- /opt/kafka/bin/kafka-topics.sh --zookeeper kafka-zookeeper:2181 --topic processor3_kafka_red --create --partitions 1 --replication-factor 1  > /dev/null
    kubectl -n kafka exec testclient -- /opt/kafka/bin/kafka-topics.sh --zookeeper kafka-zookeeper:2181 --topic sink1_kafka_green --create --partitions 1 --replication-factor 1  > /dev/null
    t=$(kubectl -n kafka exec testclient -- /opt/kafka/bin/kafka-topics.sh --zookeeper kafka-zookeeper:2181 --list | wc -l )
    sleep 1
done
