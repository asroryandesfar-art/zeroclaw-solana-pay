//! Black-box CLI integration tests: spawn the real `solpay` binary, feed it
//! flags/environment, and assert the exit code and stdout JSON. These lock the
//! machine-facing contract ZeroClaw depends on. All tests here are hermetic —
//! the only network touched is a refused connection to localhost (RPC-failure
//! case), which is deterministic and fast.

use std::process::Command;

use serde_json::Value;

const BIN: &str = env!("CARGO_BIN_EXE_solpay");
const WALLET: &str = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";
const REFERENCE: &str = "So11111111111111111111111111111111111111112";

struct Output {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run(args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(BIN);
    cmd.env_clear();
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let out = cmd.args(args).output().expect("failed to spawn solpay");
    Output {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

#[test]
fn create_url_emits_valid_json_and_exits_zero() {
    let o = run(
        &[
            "create-url",
            "--amount",
            "25",
            "--token",
            "USDC",
            "--message",
            "Table 4",
            "--reference",
            REFERENCE,
        ],
        &[("MERCHANT_WALLET", WALLET), ("SOLANA_CLUSTER", "devnet")],
    );
    assert_eq!(o.code, 0, "stderr: {}", o.stderr);

    let v: Value = serde_json::from_str(o.stdout.trim()).expect("stdout is JSON");
    assert_eq!(v["reference"], REFERENCE);
    assert_eq!(v["token"], "USDC");
    assert_eq!(v["cluster"], "devnet");
    assert_eq!(v["amount_base_units"], 25_000_000);
    assert_eq!(v["amount_ui"], "25");
    assert_eq!(v["message"], "Table 4");
    assert!(v["url"].as_str().unwrap().starts_with("solana:"));
    assert!(v["url"].as_str().unwrap().contains("spl-token="));
}

#[test]
fn render_qr_writes_a_png_and_exits_zero() {
    let mut out_path = std::env::temp_dir();
    out_path.push(format!("solpay-cli-test-{}.png", std::process::id()));
    let url = "solana:9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM?amount=25&spl-token=4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU&reference=So11111111111111111111111111111111111111112&label=Shop";

    let o = run(
        &[
            "render-qr",
            "--url",
            url,
            "--out",
            out_path.to_str().unwrap(),
        ],
        &[],
    );
    assert_eq!(o.code, 0, "stderr: {}", o.stderr);

    let v: Value = serde_json::from_str(o.stdout.trim()).unwrap();
    assert_eq!(v["format"], "png");
    assert!(v["size_bytes"].as_u64().unwrap() > 0);

    let bytes = std::fs::read(&out_path).expect("png written");
    assert_eq!(&bytes[0..4], &[0x89, 0x50, 0x4E, 0x47], "not a PNG");
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn unsupported_token_is_invalid_input() {
    let o = run(
        &[
            "create-url",
            "--amount",
            "25",
            "--token",
            "SCAM",
            "--reference",
            REFERENCE,
        ],
        &[("MERCHANT_WALLET", WALLET)],
    );
    assert_eq!(o.code, 2, "stderr: {}", o.stderr);
    assert!(o.stderr.contains("allowlist"));
}

#[test]
fn amount_over_maximum_is_invalid_input() {
    let o = run(
        &[
            "create-url",
            "--amount",
            "5000",
            "--token",
            "USDC",
            "--reference",
            REFERENCE,
        ],
        &[("MERCHANT_WALLET", WALLET), ("MAX_CHARGE", "100")],
    );
    assert_eq!(o.code, 2, "stderr: {}", o.stderr);
    assert!(o.stderr.contains("maximum"));
}

#[test]
fn bad_reference_is_invalid_input() {
    // Endpoints resolve fine; the reference is parsed and rejected before any
    // network call, so this is deterministic and offline.
    let o = run(
        &[
            "verify",
            "--reference",
            "not-a-key",
            "--amount-base-units",
            "25000000",
            "--rpc",
            "https://localhost:1",
        ],
        &[("MERCHANT_WALLET", WALLET)],
    );
    assert_eq!(o.code, 2, "stderr: {}", o.stderr);
}

#[test]
fn mainnet_without_interlock_is_config_error() {
    let o = run(
        &[
            "create-url",
            "--amount",
            "25",
            "--token",
            "USDC",
            "--reference",
            REFERENCE,
        ],
        &[
            ("MERCHANT_WALLET", WALLET),
            ("SOLANA_CLUSTER", "mainnet-beta"),
        ],
    );
    assert_eq!(o.code, 3, "stderr: {}", o.stderr);
    assert!(o.stderr.contains("ALLOW_MAINNET"));
}

#[test]
fn rpc_failure_is_transient_exit_four() {
    // Unreachable endpoint (connection refused) -> transient; SOP keeps PENDING.
    let o = run(
        &[
            "verify",
            "--reference",
            REFERENCE,
            "--amount-base-units",
            "25000000",
            "--rpc",
            "https://localhost:1",
        ],
        &[
            ("MERCHANT_WALLET", WALLET),
            ("RPC_MAX_RETRIES", "0"),
            ("RPC_TIMEOUT_MS", "800"),
        ],
    );
    assert_eq!(o.code, 4, "stderr: {}", o.stderr);
    assert!(o.stderr.contains("unavailable"));
}

#[test]
fn help_exits_zero() {
    for args in [
        vec!["--help"],
        vec!["create-url", "--help"],
        vec!["verify", "--help"],
    ] {
        let o = run(&args, &[]);
        assert_eq!(
            o.code, 0,
            "help `{args:?}` should exit 0; stderr: {}",
            o.stderr
        );
        assert!(o.stdout.contains("solpay") || o.stdout.contains("Usage"));
    }
}

/// Live devnet check (ignored by default; run with `--ignored`). A fresh,
/// never-used reference has no signatures, so the verdict is `pending`.
#[test]
#[ignore = "requires network access to Solana devnet"]
fn live_devnet_verify_is_pending_for_fresh_reference() {
    // Generate a fresh reference so getSignaturesForAddress returns empty.
    let created = run(
        &["create-url", "--amount", "25", "--token", "USDC"],
        &[("MERCHANT_WALLET", WALLET), ("SOLANA_CLUSTER", "devnet")],
    );
    assert_eq!(created.code, 0, "stderr: {}", created.stderr);
    let v: Value = serde_json::from_str(created.stdout.trim()).unwrap();
    let reference = v["reference"].as_str().unwrap().to_string();

    let o = run(
        &[
            "verify",
            "--reference",
            &reference,
            "--amount-base-units",
            "25000000",
            "--rpc",
            "https://api.devnet.solana.com",
        ],
        &[
            ("MERCHANT_WALLET", WALLET),
            ("RPC_TIMEOUT_MS", "8000"),
            ("RPC_MAX_RETRIES", "2"),
        ],
    );
    assert_eq!(o.code, 0, "stderr: {}", o.stderr);
    let out: Value = serde_json::from_str(o.stdout.trim()).unwrap();
    assert_eq!(out["status"], "pending");
}
