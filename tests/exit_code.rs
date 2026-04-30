#[test]
fn test_exit_code() {
    let bin = env!("CARGO_BIN_EXE_akave");

    // Missing required args — clap rejects the command before any network call.
    let output = std::process::Command::new(bin)
        .args(["bucket", "create"])
        .output()
        .expect("failed to spawn akave binary");
    assert_ne!(output.status.code(), Some(0), "missing args should exit non-zero");

    // All args valid but node is unreachable — port 0 is reserved and never assigned.
    let output = std::process::Command::new(bin)
        .args([
            "bucket",
            "create",
            "test-bucket",
            "--private-key",
            "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
            "--node-address",
            "http://127.0.0.1:0",
        ])
        .output()
        .expect("failed to spawn akave binary");
    assert_ne!(
        output.status.code(),
        Some(0),
        "unreachable node should exit non-zero"
    );
}
