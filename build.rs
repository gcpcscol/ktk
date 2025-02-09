// build.rs
use chrono::{DateTime, Utc};
use std::process::Command;

fn main() {
    let output = Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();
    let now_utc: DateTime<Utc> = Utc::now();
    let build_timestamp = now_utc.to_rfc3339();
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", build_timestamp);
}
