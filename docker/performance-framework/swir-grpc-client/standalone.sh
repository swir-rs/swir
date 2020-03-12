export sidecar_hostname=127.0.0.1
export sidecar_port=50051
export messages=10000
export threads=10
export client_request_topic=ProduceToKinesis
export client_response_topic=SubscribeToKinesis
export publish_type=bidi
./gradlew install
./build/install/swir-grpc-client/bin/swir-grpc-client
