source secure.sh
export RUST_LOG=info,rusoto_core=info,swir=debug,swir::si_handler=info,smdns_responder=warn,rusoto_dynamodb=info
export RUST_BACKTRACE=full
export SWIR_CONFIG_FILE=./swir.yaml
cargo run 

