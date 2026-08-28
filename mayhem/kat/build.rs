// Compile upstream's own fuzz schema (capnp/fuzz/fuzzers/test.capnp) into Rust
// via the in-tree capnpc code generator. capnpc shells out to the C++ `capnp`
// tool (apt-installed in the image). Output lands in OUT_DIR (never in-tree).
fn main() {
    capnpc::CompilerCommand::new()
        .file("../../capnp/fuzz/fuzzers/test.capnp")
        .src_prefix("../../capnp/fuzz/fuzzers")
        .run()
        .expect("compiling test.capnp schema");
}
