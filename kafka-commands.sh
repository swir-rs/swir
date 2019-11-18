kafkacat -b 127.0.0.1:9092 -C -t Request
kafkacat -P  -b localhost:9092 -H "header1=header value" -H "nullheader" -H "emptyheader=" -H "header1=duplicateIsOk" -t Response


docker pull confluentinc/cp-kafkacat
docker run -ti --network="docker_default" confluentinc/cp-kafkacat kafkacat -C -b docker_kafka_1:9092 -t Request
docker run -ti --network="docker_default" confluentinc/cp-kafkacat kafkacat -P -b docker_kafka_1:9092 -t Response

