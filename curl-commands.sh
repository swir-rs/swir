curl -k --cacert demoCA/cacert.pem -d '{"key1":"value1", "key2":"value2"}' -H "Content-Type: application/json" -X POST https://localhost:8443/publish
curl -v -d '{"key1":"value1", "key2":"value2"}' -H "Content-Type: application/json" -X POST http://localhost:8080/publish
curl -v -d '{"endpoint":{"url":"http://127.0.0.1:8090/response"}}' -H "Content-Type: application/json" -X POST http://localhost:8080/subscribe

docker run --network docker_swir-net -it curlimages/curl -v -d '{"endpoint":{"url":"http://127.0.0.1:8090/response"}}' -H "Content-Type: application/json" -X POST http://docker_swir_1:8080/subscribe
docker run --network docker_swir-net -it curlimages/curl -v -d '{"messages":10000, "threads":4, "sidecarUrl":"http://127.0.0.1:8080/publish"}' -H "Content-Type: application/json" -X POST http://docker_swir_1:8090/test
