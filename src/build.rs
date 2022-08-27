extern crate bindgen;

fn main() {
    println!("cargo:rustc-link-search=lib");

    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=IOTCAPIs_ALL");
    }
    #[cfg(not(target_os = "macos"))]
    {
        println!("cargo:rustc-link-lib=IOTCAPIs");
        println!("cargo:rustc-link-lib=AVAPIs");
    }

    println!("cargo:rerun-if-changed=headers/wrapper.h");

    let bindings = bindgen::Builder::default()
        .header("headers/wrapper.h")
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file("src/bindings.rs")
        .expect("Couldn't write bindings!");
}
