use std::env;

fn main() {
    // Check if CONFIG_PATH is set; if not, use a default value
    let config_path = env::var("ATTOMAIL_CONFIG_PATH").unwrap_or_else(|_| "/etc/attomail.conf".to_string());

    // Tell Cargo to rerun this build script if the environment variable changes
    println!("cargo:rerun-if-env-changed=ATTOMAIL_CONFIG_PATH");

    // Pass the value to the Rust code by emitting a cargo instruction
    println!("cargo:rustc-env=ATTOMAIL_CONFIG_PATH={}", config_path);
}

