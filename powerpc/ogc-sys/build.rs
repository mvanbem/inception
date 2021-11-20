use std::env;
use std::path::PathBuf;

use bindgen::RustTarget;

fn main() {
    match (cfg!(feature = "gamecube"), cfg!(feature = "wii")) {
        (true, true) | (false, false) => {
            panic!("Either the 'gamecube' or 'wii' feature must be enabled, and not both.");
        }
        _ => (),
    }

    let devkitpro_path = env::var("DEVKITPRO").expect("devkitPro is needed to use this crate");
    let devkitppc_path =
        env::var("DEVKITPPC").expect("devkitPro's devkitPPC is needed to use this crate");
    println!(
        "cargo:rustc-link-search=native={}/devkitPPC/powerpc-eabi/lib",
        devkitpro_path,
    );
    #[cfg(feature = "gamecube")]
    println!(
        "cargo:rustc-link-search=native={}/libogc/lib/cube",
        devkitpro_path,
    );
    #[cfg(feature = "wii")]
    println!(
        "cargo:rustc-link-search=native={}/libogc/lib/wii",
        devkitpro_path,
    );

    println!("cargo:rustc-link-lib=static=c");
    println!("cargo:rustc-link-lib=static=sysbase");
    println!("cargo:rustc-link-lib=static=m");
    println!("cargo:rustc-link-lib=static=ogc");

    #[cfg(feature = "wii")]
    {
        println!("cargo:rustc-link-lib=static=bte");
        println!("cargo:rustc-link-lib=static=wiiuse");
    }

    println!("cargo:rerun-if-changed=wrapper.h");
    let builder = bindgen::Builder::default()
        .header("wrapper.h")
        .rust_target(RustTarget::Nightly)
        .use_core()
        .layout_tests(false)
        .ctypes_prefix("::libc")
        .prepend_enum_name(false)
        // .disable_untagged_union()
        .blocklist_type("u(8|16|32|64|128)")
        .blocklist_type("i(8|16|32|64|128)")
        .blocklist_type("f(32|64)")
        .no_debug("_tmd")
        .no_debug("_tmdview")
        .clang_arg("--target=powerpc-none-eabi")
        .clang_arg("-mcpu=750")
        .clang_arg("-mfloat-abi=hard")
        .clang_arg(format!("--sysroot={}/powerpc-eabi", devkitppc_path))
        .clang_arg(format!("-isystem{}/powerpc-eabi/include", devkitppc_path))
        .clang_arg(format!("-I{}/libogc/include", devkitpro_path))
        .clang_arg("-Wno-macro-redefined")
        .clang_arg("-Wno-incompatible-library-redeclaration")
        .clang_arg("-DGEKKO")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks));

    #[cfg(feature = "wii")]
    let builder = builder.clang_arg("-DHW_RVL");

    let bindings = builder.generate().expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
