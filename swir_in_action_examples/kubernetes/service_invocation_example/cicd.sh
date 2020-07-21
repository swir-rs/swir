#!/bin/bash
cwd=$(pwd)
cd ..
./cicd.sh
cd $cwd
./build_clients.sh
./deploy_example.sh

printf "\n\n************************************\n\n"
printf "\n\nTo check logs run:  \n\n"
printf "\n./display_books_logs.sh\n"
printf "\n./display_helpdesk_logs.sh\n"
printf "\n\n************************************\n\n"

../tracing_dashboard.sh


