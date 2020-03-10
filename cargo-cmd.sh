export RUST_LOG=INFO,rusoto_core=INFO,swir=DEBUG,aws_lock_client=trace
export RUST_BACKTRACE=full
export SWIR_FILE_NAME=./swir.yaml; cargo run 

