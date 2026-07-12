use std::process::Command;

#[test]
fn help_flag_prints_usage() {
    let out = Command::new(env!("CARGO_BIN_EXE_llm-gateway"))
        .arg("--help")
        .output()
        .expect("spawn gateway binary");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("grant-platform-admin"),
        "help should list the subcommand"
    );
}

// NOTE: An end-to-end integration test that spawns the binary against a real
// Postgres instance and verifies the user's platform_role is flipped was
// considered but deferred — see the Task 6 plan. It would require:
//   * A test Postgres reachable via DATABASE_URL (none configured here).
//   * Threading DATABASE_URL + a temp config.toml into a subprocess.
//   * Coordinating the sqlx::test pool with the spawned binary.
// The help-flag test above covers the CLI surface; the underlying storage
// behavior (`set_user_platform_role`) is already covered by storage-layer
// integration tests added in Task 1.