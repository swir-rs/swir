docker-compose -f docker-compose-swir.yaml -p pf down --remove-orphans
docker-compose -f docker-compose-infr.yaml -p docker down --remove-orphans
