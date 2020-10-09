docker-compose -p app -f docker-compose-example-pubsub.yaml down --remove-orphans
docker-compose -f docker-compose-infr.yml -p docker down --remove-orphans
