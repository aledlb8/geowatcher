fn main() {
    // Exposed as TARGET_TRIPLE so the app can find its per-platform sidecar.
    if let Ok(target) = std::env::var("TARGET") {
        println!("cargo:rustc-env=TARGET_TRIPLE={target}");
    }
    tauri_build::build();
}
