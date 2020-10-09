#!/bin/bash
x=0
while [ $x -le 0 ]
do
    printf "Checking for Source logs"
    x=$(kubectl get po -n $1 | grep "source" | grep "3/3" | grep "Running" |wc -l)
    if [[ $x -le 0 ]]; then	
	sleep 5
    fi    
done

id=$(kubectl get po -n $1 | grep source |  grep "3/3" | grep "Running" | awk '{ print $1 }')
kubectl logs -n $1 $id -c client


