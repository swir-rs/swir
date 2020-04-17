fn main() {
    tonic_build::compile_protos("grpc_api/client_api.proto")
        .unwrap_or_else(|e| panic!("Failed to compile protos {:?}", e));
    tonic_build::compile_protos("grpc_api/internal_api.proto")
        .unwrap_or_else(|e| panic!("Failed to compile internal protos {:?}", e));
}
