fn main() {
    // Ensure Cargo recompiles when compile-time env vars change
    println!("cargo:rerun-if-env-changed=API_PRIVATE_KEY");
    println!("cargo:rerun-if-env-changed=PLAYBOOK_PUBLIC_KEY");
    println!("cargo:rerun-if-env-changed=USE_PRODUCTION");
    println!("cargo:rerun-if-env-changed=USE_SANDBOX");
    tauri_build::build()
}
