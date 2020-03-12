source secure.sh
export RUST_LOG=INFO,rusoto_core=INFO,swir=DEBUG,aws_lock_client=INFO
export RUST_BACKTRACE=full
export SWIR_CONFIG_FILE=./swir_aws.yaml
cargo run 

