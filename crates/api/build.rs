fn main() {
    let version = std::env::var("LLM_GATEWAY_VERSION")
        .ok()
        .or_else(|| {
            let output = std::process::Command::new("git")
                .args(["describe", "--tags", "--abbrev=0"])
                .output()
                .ok()?;
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        });

    if let Some(v) = version {
        println!("cargo:rustc-env=GIT_VERSION={}", v);
    }
}
