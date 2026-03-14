use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let out = &PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set"));
    File::create(out.join("memory.x"))
        .expect("create memory.x")
        .write_all(include_bytes!("memory.x"))
        .expect("write memory.x");
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=memory.x");
    if std::path::Path::new("defmt.x").exists() {
        println!("cargo:rerun-if-changed=defmt.x");
    }
}
