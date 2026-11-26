use std::env;

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    let target = env::var("TARGET").expect("Cargo must provide the build target");
    println!("cargo::rustc-env=JDA_BUILD_TARGET={target}");
}
