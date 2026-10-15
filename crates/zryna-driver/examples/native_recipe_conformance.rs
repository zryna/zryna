//! Explicitly invoked production-API native recipe conformance; never an auto-skipped test.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "native_recipe_conformance/host.rs"]
mod host;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    host::run()
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
fn main() {
    eprintln!("native recipe conformance requires the admitted Linux x86-64 host");
    std::process::exit(1);
}
