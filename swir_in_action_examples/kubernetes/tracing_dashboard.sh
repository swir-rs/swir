#!/bin/bash

while [ -z $dashboard_ip ]
do    
    maybe_ip=$(kubectl -n $1 get ingress | grep jaeger-allinone-query)    
    words=$(echo "$maybe_ip" | wc -w )
    if [[ $words -eq 6 ]]; then
	dashboard_ip=$(echo "$maybe_ip" | awk '{ print $4}')	
    fi
    if [[ -z $dashboard_ip ]]; then
	    echo "Waiting for Dashboard IP to be set "
	    sleep 5
    fi
done

dashboard_url=http://$dashboard_ip:80

printf "Dashboard can be found at "
printf "\n\n$dashboard_url\n\n"

