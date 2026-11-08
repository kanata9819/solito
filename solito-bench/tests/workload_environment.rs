use std::process::Command;

#[test]
fn rejects_zero_workload_settings_from_environment() {
    for setting in ["SOLITO_BENCH_FPS", "SOLITO_BENCH_DURATION_SECONDS"] {
        let output = Command::new(env!("CARGO_BIN_EXE_solito-bench"))
            .env("SOLITO_BENCH_WORKLOAD", "1")
            .env("SOLITO_BENCH_MODE", "full")
            .env("SOLITO_BENCH_FPS", "60")
            .env("SOLITO_BENCH_DURATION_SECONDS", "1")
            .env(setting, "0")
            .env_remove("SOLITO_BENCH_READY_FILE")
            .output()
            .unwrap();

        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("must be greater than zero"));
    }
}
