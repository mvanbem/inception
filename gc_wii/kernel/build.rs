use std::env;
use std::path::Path;

fn main() {
    println!("cargo:rustc-link-arg=-nostartfiles");

    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    let linker_script = crate_dir.join("link.x");
    println!("cargo:rustc-link-arg=-T{}", linker_script.display());
    println!("cargo:rerun-if-changed={}", linker_script.display());

    let bin_dir = crate_dir.join("bin");
    let libstart = crate_dir.join("bin/libstart.a");
    println!("cargo:rustc-link-lib=static=start");
    println!("cargo:rustc-link-search={}", bin_dir.display());
    println!("cargo:rerun-if-changed={}", libstart.display());
}
