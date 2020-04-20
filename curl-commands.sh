#NATS and ssl ok
curl -k --cacert demoCA/cacert.pem -d '{"key1":"value1", "key2":"value2"}' -H "Content-Type: application/json" -H "topic: ProduceToAppB" -H "x-correlation-id:ksjdskjdksjdksjd" -X POST https://localhost:8443/publish
#Kafka and ssl ok
curl -k --cacert demoCA/cacert.pem -d '{"key1":"value1", "key2":"value2"}' -H "Content-Type: application/json" -H "topic: ProduceToAppA" -H "x-correlation-id:ksjdskjdksjdksjd" -X POST https://localhost:8443/publish

#Kafka ok
curl -v -d '{"key1":"value1", "key2":"value2"}' -H "Content-Type: application/json" -H "topic: ProduceToAppA" -H "x-correlation-id:ksjdskjdksjdksjd" -X POST http://localhost:8080/publish
curl -v -d '{"endpoint":{"url":"http://127.0.0.1:8090/response"},"client_topic":"SubscribeToAppA"}' -H "Content-Type: application/json" -X POST  http://localhost:8080/subscribe
#Kafka error
curl -v -d '{"key1":"value1", "key2":"value2"}' -H "Content-Type: application/json" -H "topic: SubscribeToAppA" -H "x-correlation-id:ksjdskjdksjdksjd" -X POST http://localhost:8080/publish
curl -v -d '{"endpoint":{"url":"http://127.0.0.1:8090/response"},"client_topic":"ProduceToAppA"}' -H "Content-Type: application/json" -H "x-correlation-id:ksjdskjdksjdksjd" -X POST  http://localhost:8080/subscribe


#Nats ok
curl -v -d '{"key1":"value1", "key2":"value2"}' -H "Content-Type: application/json" -H "topic: ProduceToAppB" -H "x-correlation-id:ksjdskjdksjdksjd" -X POST http://localhost:8080/publish

head -c 1k </dev/urandom >testfile
curl -v -H "Content-Type:application/octet-stream" -H "topic: ProduceToAppB" -H "x-correlation-id:ksjdskjdksjdksjd" --data-binary @testfile http://localhost:8080/publish

curl -v -d '{"endpoint":{"url":"http://127.0.0.1:8090/response"},"client_topic":"SubscribeToAppB"}' -H "Content-Type: application/json" -X POST  http://localhost:8080/subscribe
#Nats error
curl -v -d '{"key1":"value1", "key2":"value2"}' -H "Content-Type: application/json" -H "topic: SubscribeToAppB" -X POST http://localhost:8080/publish
curl -v -d '{"endpoint":{"url":"http://127.0.0.1:8090/response"},"client_topic":"ProduceToAppB"}' -H "Content-Type: application/json" -X POST  http://localhost:8080/subscribe

docker run --network docker_swir-net -it curlimages/curl -v -d '{"endpoint":{"url":"http://127.0.0.1:8090/response"}}' -H "Content-Type: application/json" -X POST http://docker_swir_1:8080/subscribe
docker run --network docker_swir-net -it curlimages/curl -v -d '{"messages":10000, "threads":4, "sidecarUrl":"http://127.0.0.1:8080/publish"}' -H "Content-Type: application/json" -X POST http://docker_swir_1:8090/test


#Redis ok
curl -v -d '{"key1":"value1", "key2":"value2"}' -H "Content-Type: application/json" -H "x-database-key: dfkdkfjdkjf" -H "x-database-name: my_db_store1" -H "x-correlation-id:ksjdskjdksjdksjd" -X POST http://localhost:8080/store
curl -v -d '{"key1":"value1", "key2":"value2"}' -H "Content-Type: application/json" -H "x-database-key: dfkdkfjdkjf" -H "x-database-name: my_db_store1" -H "x-correlation-id:ksjdskjdksjdksjd" -X POST http://localhost:8080/retrieve



curl -v -d '{"key1":"value1", "key2":"value2"}' -H "Content-Type: application/json" -H "topic: ProduceToAppA" -H "x-correlation-id:ksjdskjdksjdksjd" -X POST http://dawid-gigner-3rd.local:8080/publish


curl -v -d '{"method":"POST", "request_target":"/blah/id?value=dfkhdfhkj", "headers":{"Content-type":"Application-json","identity":"bummer"}, "payload":"aGVyZSBiZSBkcmFnb25zLi4uIGxpa2UgZHJhZ29uIGJhbGwK"}' -H "Content-Type: application/json"  -H "x-correlation-id:ksjdskjdksjdksjd" -X POST http://localhost:8080/serviceinvocation/invoke/service_1

{"ooo":"bllll","dfjhdkjfk":"dskfjkdjf","dfjkdjfkd":"kfjkdjfkdjk"}

curl -v -d '{"method":"POST", "request_target":"/blah/id?value=dfkhdfhkj", "headers":{"Content-type":"Application-json","identity":"bummer"}, "payload":"dflk:sdfjkfj"}"}' -H "Content-Type: application/json"  -H "x-correlation-id:ksjdskjdksjdksjd" -X POST http://localhost:8080/serviceinvocation/invoke/service_1

