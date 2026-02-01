fn main() {
    // Git commit hash を環境変数として埋め込む
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let git_hash = String::from_utf8_lossy(&output.stdout);
            println!("cargo:rustc-env=GIT_HASH={}", git_hash.trim());
        }
    }

    tauri_build::build()
}
