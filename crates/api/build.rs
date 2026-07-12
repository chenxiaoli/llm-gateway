fn main() {
    // The version string shown by /api/v1/version comes from one of:
    //   1. LLM_GATEWAY_VERSION env var at build time (set by Dockerfile + CI release workflow)
    //   2. concat!("v", CARGO_PKG_VERSION) at compile time (local dev — see mod.rs)
    //
    // We deliberately do NOT fall back to `git describe` here: release.sh tags on
    // `main`, but local dev builds run against `develop`, where the most recent
    // reachable tag can be arbitrarily old (we saw v1.8.0 reported while Cargo.toml
    // was already at 2.0.0). Letting Cargo.toml be the source of truth in dev is
    // both simpler and correct.
    if let Ok(v) = std::env::var("LLM_GATEWAY_VERSION") {
        println!("cargo:rustc-env=GIT_VERSION={}", v);
    }
}
