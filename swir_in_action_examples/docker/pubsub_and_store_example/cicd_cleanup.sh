docker-compose -p app -f docker-compose-example-applications.yaml down --remove-orphans
docker-compose -p app -f docker-compose-example-sidecars.yaml down --remove-orphans
docker-compose -f docker-compose-infr.yml -p docker down --remove-orphans
