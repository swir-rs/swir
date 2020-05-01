fn main() {
    tonic_build::compile_protos("grpc_api/common_structs.proto").unwrap_or_else(|e| panic!("Failed to compile common_structs {:?}", e));
    tonic_build::compile_protos("grpc_api/client_api.proto").unwrap_or_else(|e| panic!("Failed to compile client_api {:?}", e));
    tonic_build::compile_protos("grpc_api/internal_api.proto").unwrap_or_else(|e| panic!("Failed to compile internal_api {:?}", e));
}
