use std::env;

fn main() {
    if env::var_os("ZTRACING").is_some() {
        println!("cargo::rustc-cfg=a_tracing");
    }
    if env::var_os("ZTRACING_WITH_MEMORY").is_some() {
        println!("cargo::rustc-cfg=a_tracing");
        println!("cargo::rustc-cfg=a_tracing_with_memory");
    }
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-env-changed=ZTRACING");
    println!("cargo::rerun-if-env-changed=ZTRACING_WITH_MEMORY");
}
