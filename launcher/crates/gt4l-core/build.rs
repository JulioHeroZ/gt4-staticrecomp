#[path = "build_support/release_label.rs"]
mod release_label;

fn main() {
    println!("cargo:rerun-if-env-changed=GT4_RELEASE_LABEL");
    println!("cargo:rerun-if-changed=build_support/release_label.rs");
    let numeric = std::env::var("CARGO_PKG_VERSION").expect("Cargo supplies the package version");
    let label = std::env::var("GT4_RELEASE_LABEL").unwrap_or_default();
    let display = release_label::display_version(&numeric, &label).expect("invalid release label");
    println!("cargo:rustc-env=GT4_LAUNCHER_DISPLAY_VERSION={display}");
    println!("cargo:rustc-env=GT4_RELEASE_LABEL={label}");
}
