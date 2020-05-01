#necessary certs for HTTPs but only on the first run
if [ ! -f swir.CA.key ]; then
    ./generate-cert.sh
fi


cargo build --release --features="with_nats"
docker build . --build-arg executable=target/release/swir --build-arg swir_config=docker/performance-framework/swir.yaml -t swir:v3


#see more
# docker/performance-framework/cicd.sh
# docker/solution-example/cicd.sh
# docker/solution-example_aws/cicd.sh
