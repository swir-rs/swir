docker-compose -f docker-compose-swir.yml -p pf down --remove-orphans
docker-compose -f docker-compose-infr.yml -p docker down --remove-orphans
