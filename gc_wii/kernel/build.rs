use std::env;
use std::path::Path;

fn main() {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    println!("cargo:rustc-link-arg=-nostartfiles");

    let linker_script = crate_dir.join("link.x");
    println!("cargo:rustc-link-arg=-T{}", linker_script.display());
    println!("cargo:rerun-if-changed={}", linker_script.display());

    let build_dir = crate_dir.join("../../build");
    let libstart = build_dir.join("libstart.a");
    println!("cargo:rustc-link-lib=static=start");
    println!("cargo:rustc-link-search={}", build_dir.display());
    println!("cargo:rerun-if-changed={}", libstart.display());
}
