//! Compiles the vendored ymfm OPL sources and the C shim.

fn main() {
    let ymfm = std::path::Path::new("../../third_party/ymfm");
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .opt_level(2)
        .warnings(false)
        .include(ymfm)
        .file("src/shim.cpp")
        .file(ymfm.join("ymfm_opl.cpp"))
        .file(ymfm.join("ymfm_adpcm.cpp"))
        .file(ymfm.join("ymfm_pcm.cpp"))
        .compile("ymfm");
    println!("cargo:rerun-if-changed=src/shim.cpp");
    println!("cargo:rerun-if-changed=../../third_party/ymfm");
}
