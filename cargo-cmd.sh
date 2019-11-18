#For kafka
cargo build
cargo run --package rustycar --bin rustycar -- -a 127.0.0.1  -b 127.0.0.1:9092 -s Request -r Response -g rustycar

#For NATS
cargo build build --features="with_nats"
cargo run --features "with_nats" --package rustycar --bin rustycar -- -a 127.0.0.1  -b 127.0.0.1:4222 -s Request -r Response -g rustycar
