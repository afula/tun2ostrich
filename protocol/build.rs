fn main() {
    let protoc_bin_path = protoc_bin_vendored::protoc_bin_path().unwrap();

    let protos = ["proto/notification.proto"];

    protobuf_codegen::Codegen::new()
        .protoc_path(&protoc_bin_path)
        .include("proto")
        .inputs(&protos)
        .out_dir("src/generated")
        .run_from_script();

    // prost_build::Config::new()
    //     .compile_protos(&["proto/notification.proto"], &["proto"])
    //     .unwrap();
    for proto in &protos {
        println!("cargo:rerun-if-changed={}", proto);
    }
}
