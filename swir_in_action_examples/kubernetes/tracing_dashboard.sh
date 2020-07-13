#!/bin/bash

dashboard_ip=$(kubectl -n swir get ingress | grep jaeger-allinone-query | awk '{ print $4 }')      
dashboard_url=http://$dashboard_ip:80

printf "Dashboard can be found at "
printf "\n\n$dashboard_url\n\n"

