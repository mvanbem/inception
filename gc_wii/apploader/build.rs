use std::path::Path;

fn main() {
    let crate_dir = env!("CARGO_MANIFEST_DIR");
    let linker_script = Path::new(crate_dir).join("link.x");

    println!("cargo:rustc-link-arg=-nostartfiles");
    println!("cargo:rustc-link-arg=-T{}", linker_script.display());
}
