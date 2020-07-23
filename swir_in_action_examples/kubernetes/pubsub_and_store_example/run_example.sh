#!/bin/bash
cwd=$(pwd)
cd ..
./setup.sh
cd $cwd
./deploy_infrastructure.sh
./build_clients.sh
./deploy_example.sh

printf "\n\n************************************\n\n"
printf "\n\nTo check logs run: \n\n "
printf "\n./display_source_logs.sh\n"
printf "\n./display_sink_logs.sh\n"
printf "\n\n************************************\n\n"

../tracing_dashboard.sh


