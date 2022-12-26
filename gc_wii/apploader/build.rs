use std::path::Path;

fn main() {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    println!("cargo:rustc-link-arg=-nostartfiles");

    let linker_script = crate_dir.join("link.x");
    println!("cargo:rustc-link-arg=-T{}", linker_script.display());
    println!("cargo:rerun-if-changed={}", linker_script.display());
}
