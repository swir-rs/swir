#!/bin/bash
x=0
while [ $x -le 0 ]
do
    echo "Checking for Magazines logs"
    x=$(kubectl get po -n swir | grep "magazines" | grep "3/3" | grep "Running" |wc -l)
    if [[ $x -le 0 ]]; then	
	sleep 5
    fi    
done

id=$(kubectl get po -n swir | grep magazines |  grep "3/3" | grep "Running" | awk '{ print $1 }')
kubectl logs -n swir $id -c client


