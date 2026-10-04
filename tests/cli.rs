//! Black-box tests of the `p10-replay` binary against three signed public S2b fixtures.
//! The fixtures are test data only; expected values come from the S2b tag `v0.1.1-s2b-public`.

mod support;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

use serde_json::Value;
use support::sha256_hex;

const VERIFIED_REQ: &str = "tests/fixtures/verified/request.cbor";
const INVALID_REQ: &str = "tests/fixtures/invalid/request.cbor";
const UNAVAILABLE_REQ: &str = "tests/fixtures/unavailable/request.cbor";

const VERIFIED_REQ_SHA: &str = "b22b7e70c6750d19c2278275b49d8eb9092832e2d833df2ba08bfb4fdacbbae9";
const INVALID_REQ_SHA: &str = "807840396ae229047f59a053a716fa6857d0d6abe109725faaca446848528222";
const UNAVAILABLE_REQ_SHA: &str =
    "4f3596aafc0515459b91e766e3c7d9383797de2c9e7a5d35e650ff3cad2e7300";
const VERIFIED_TB_SHA: &str = "731452bded7d86b74985dfd449f8ece8c2fc600cec5fea8d92d873cd60a6c865";

fn fixture(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// Fresh empty directory per test.
fn workdir(name: &str) -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "{name}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_p10-replay"))
}

fn run(args: &[&str]) -> Output {
    bin().args(args).output().unwrap()
}

fn run_stdin(args: &[&str], input: &[u8]) -> Output {
    let mut child = bin()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

fn stdout_json(o: &Output) -> Value {
    let text = String::from_utf8(o.stdout.clone()).unwrap();
    assert!(
        text.ends_with('\n') && !text.ends_with("\n\n"),
        "exactly one trailing newline: {text:?}"
    );
    assert_eq!(
        text.matches('\n').count(),
        1,
        "stdout must be exactly one JSON line: {text:?}"
    );
    serde_json::from_str(&text).unwrap()
}

fn entries(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    v.sort();
    v
}

fn is_hex64(v: &Value) -> bool {
    v.as_str()
        .is_some_and(|s| s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
}

// ------------------------------------------------------------------------------------------ fixtures

#[test]
fn fixtures_match_recorded_hashes() {
    for (rel, want) in [
        (VERIFIED_REQ, VERIFIED_REQ_SHA),
        (INVALID_REQ, INVALID_REQ_SHA),
        (UNAVAILABLE_REQ, UNAVAILABLE_REQ_SHA),
    ] {
        assert_eq!(sha256_hex(&fs::read(fixture(rel)).unwrap()), want, "{rel}");
    }
    let sums = fs::read_to_string(fixture("tests/fixtures/SHA256SUMS")).unwrap();
    for line in sums.lines() {
        let (hash, path) = line.split_once("  ").unwrap();
        assert_eq!(
            sha256_hex(&fs::read(fixture(&format!("tests/fixtures/{path}"))).unwrap()),
            hash,
            "{path}"
        );
    }
    assert_eq!(sums.lines().count(), 3);
}

// ------------------------------------------------------------------------------------------ help / usage

#[test]
fn top_level_help_and_version_go_to_stdout() {
    let o = run(&["--help"]);
    assert_eq!(o.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&o.stdout).contains("USAGE:"));
    assert!(o.stderr.is_empty());

    let o = run(&["--version"]);
    assert_eq!(o.status.code(), Some(0));
    let out = String::from_utf8(o.stdout).unwrap();
    assert!(out.starts_with("p10-replay 0.1.0\n"), "{out}");
    assert!(out.contains("8e89f9a4192bdd0ab280556701dc595eb55539d2"));
    assert!(o.stderr.is_empty());
}

#[test]
fn verify_help_goes_to_stderr() {
    let o = run(&["verify", "--help"]);
    assert_eq!(o.status.code(), Some(0));
    assert!(o.stdout.is_empty());
    assert!(String::from_utf8_lossy(&o.stderr).contains("--transcript-out"));
}

#[test]
fn usage_errors_exit_64_with_clean_stdout() {
    let cases: &[&[&str]] = &[
        &[],
        &["frobnicate"],
        &["--bogus"],
        &["verify"],
        &["verify", "--request", VERIFIED_REQ],
        &["verify", "--transcript-out", "t.bin"],
        &[
            "verify",
            "--request",
            VERIFIED_REQ,
            "--transcript-out",
            "t.bin",
            "--unknown",
        ],
        &[
            "verify",
            "--request",
            VERIFIED_REQ,
            "--request",
            VERIFIED_REQ,
            "--transcript-out",
            "t.bin",
        ],
        &[
            "verify",
            "--request",
            VERIFIED_REQ,
            "--transcript-out",
            "t.bin",
            "--transcript-out",
            "u.bin",
        ],
        &[
            "verify",
            "--request",
            VERIFIED_REQ,
            "--transcript-out",
            "t.bin",
            "--result-out",
            "-",
            "--result-out",
            "-",
        ],
        &[
            "verify",
            "--request",
            VERIFIED_REQ,
            "--transcript-out",
            "t.bin",
            "--force",
            "--force",
        ],
        &[
            "verify",
            "--request",
            VERIFIED_REQ,
            "--transcript-out",
            "t.bin",
            "stray",
        ],
        &["verify", "--request", VERIFIED_REQ, "--transcript-out"],
        &["verify", "--request", VERIFIED_REQ, "--transcript-out", "-"],
    ];
    for args in cases {
        let o = run(args);
        assert_eq!(o.status.code(), Some(64), "{args:?}");
        assert!(o.stdout.is_empty(), "{args:?}");
        assert!(!o.stderr.is_empty(), "{args:?}");
    }
    assert!(fs::metadata("t.bin").is_err() && fs::metadata("u.bin").is_err());
}

// ------------------------------------------------------------------------------------------ Verified

fn check_verified(o: &Output, tb: &Path) -> Value {
    assert_eq!(
        o.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(o.stderr.is_empty(), "no operational noise on success");
    let bytes = fs::read(tb).unwrap();
    assert_eq!(sha256_hex(&bytes), VERIFIED_TB_SHA);
    let j = stdout_json(o);
    assert_eq!(j["schema"], "P10ReplayCliResultV0");
    assert_eq!(j["status"], "verified");
    assert_eq!(j["transcript"]["bytes"], bytes.len());
    assert_eq!(j["transcript"]["sha256"], VERIFIED_TB_SHA);
    let ev = &j["evidence_summary"];
    assert!(ev["checkpoint_size"].is_u64());
    assert!(is_hex64(&ev["root_digest"]));
    assert!(is_hex64(&ev["log_identity_digest"]));
    assert!(ev["statement_count"].is_u64());
    assert!(ev["subject_observation_count"].is_u64());
    assert!(ev["unreadable_row_count"].is_u64());
    assert_eq!(j.as_object().unwrap().len(), 4);
    j
}

#[test]
fn verified_from_file_writes_exact_transcript_and_json_to_stdout() {
    let d = workdir("verified-file");
    let tb = d.join("tB.bin");
    let o = run(&[
        "verify",
        "--request",
        s(&fixture(VERIFIED_REQ)),
        "--transcript-out",
        s(&tb),
    ]);
    let j = check_verified(&o, &tb);
    assert_eq!(j["evidence_summary"]["checkpoint_size"], 2);
    assert_eq!(j["evidence_summary"]["unreadable_row_count"], 1);
    assert_eq!(entries(&d), ["tB.bin"], "no temporary files left behind");
}

#[test]
fn verified_from_stdin() {
    let d = workdir("verified-stdin");
    let tb = d.join("tB.bin");
    let input = fs::read(fixture(VERIFIED_REQ)).unwrap();
    let o = run_stdin(
        &["verify", "--request", "-", "--transcript-out", s(&tb)],
        &input,
    );
    check_verified(&o, &tb);
}

#[test]
fn result_json_to_file_leaves_stdout_empty() {
    let d = workdir("result-file");
    let (tb, res) = (d.join("tB.bin"), d.join("result.json"));
    let o = run(&[
        "verify",
        "--request",
        s(&fixture(VERIFIED_REQ)),
        "--transcript-out",
        s(&tb),
        "--result-out",
        s(&res),
    ]);
    assert_eq!(o.status.code(), Some(0));
    assert!(o.stdout.is_empty() && o.stderr.is_empty());
    let text = fs::read_to_string(&res).unwrap();
    assert!(text.ends_with('\n') && !text.ends_with("\n\n"));
    let j: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(j["status"], "verified");
    assert_eq!(j["transcript"]["sha256"], VERIFIED_TB_SHA);
    assert_eq!(entries(&d), ["result.json", "tB.bin"]);
}

#[test]
fn explicit_dash_result_out_is_stdout() {
    let d = workdir("dash");
    let tb = d.join("tB.bin");
    let o = run(&[
        "verify",
        "--request",
        s(&fixture(VERIFIED_REQ)),
        "--transcript-out",
        s(&tb),
        "--result-out",
        "-",
    ]);
    check_verified(&o, &tb);
}

// ------------------------------------------------------------------------------------------ Invalid / Unavailable

#[test]
fn invalid_exits_2_and_creates_no_transcript() {
    let d = workdir("invalid");
    let tb = d.join("tB.bin");
    let o = run(&[
        "verify",
        "--request",
        s(&fixture(INVALID_REQ)),
        "--transcript-out",
        s(&tb),
    ]);
    assert_eq!(o.status.code(), Some(2));
    assert!(o.stderr.is_empty());
    let j = stdout_json(&o);
    assert_eq!(j["schema"], "P10ReplayCliResultV0");
    assert_eq!(j["status"], "invalid");
    assert_eq!(j["primary"]["phase"], "p8");
    assert_eq!(j["primary"]["severity"], "invalid");
    assert_eq!(j["primary"]["reason"], "target_kind_mismatch");
    assert_eq!(j["primary"]["parameters"], serde_json::json!({}));
    assert!(j["diagnostics"].is_array());
    assert!(
        entries(&d).is_empty(),
        "no transcript and no temporary file"
    );
}

#[test]
fn unavailable_exits_3_and_creates_no_transcript() {
    let d = workdir("unavailable");
    let tb = d.join("tB.bin");
    let o = run(&[
        "verify",
        "--request",
        s(&fixture(UNAVAILABLE_REQ)),
        "--transcript-out",
        s(&tb),
    ]);
    assert_eq!(o.status.code(), Some(3));
    assert!(o.stderr.is_empty());
    let j = stdout_json(&o);
    assert_eq!(j["status"], "unavailable");
    assert_eq!(j["primary"]["phase"], "p4");
    assert_eq!(j["primary"]["severity"], "unavailable");
    assert_eq!(j["primary"]["class"], "material");
    assert_eq!(j["primary"]["reason"], "missing_index");
    assert_eq!(
        j["primary"]["parameters"],
        serde_json::json!({ "first_missing": 4 })
    );
    assert!(entries(&d).is_empty());
}

// ------------------------------------------------------------------------------------------ transport / I/O

#[test]
fn malformed_cbor_exits_65_transport_error() {
    for garbage in [&b"\xff\xff not cbor"[..], &b""[..], &b"\x80"[..]] {
        let d = workdir("malformed");
        let (req, tb) = (d.join("req.cbor"), d.join("tB.bin"));
        fs::write(&req, garbage).unwrap();
        let o = run(&["verify", "--request", s(&req), "--transcript-out", s(&tb)]);
        assert_eq!(o.status.code(), Some(65), "{garbage:?}");
        let j = stdout_json(&o);
        assert_eq!(j["schema"], "P10ReplayCliResultV0");
        assert_eq!(j["status"], "transport_error");
        assert_eq!(j["code"], "request_decode_failed");
        assert!(j["message"].as_str().is_some_and(|m| !m.is_empty()));
        assert!(
            j.get("primary").is_none(),
            "must not be mislabeled as a ReplayResult"
        );
        assert_eq!(entries(&d), ["req.cbor"]);
    }
}

#[test]
fn malformed_cbor_from_stdin_exits_65() {
    let d = workdir("malformed-stdin");
    let o = run_stdin(
        &[
            "verify",
            "--request",
            "-",
            "--transcript-out",
            s(&d.join("tB.bin")),
        ],
        b"junk",
    );
    assert_eq!(o.status.code(), Some(65));
    assert_eq!(stdout_json(&o)["status"], "transport_error");
    assert!(entries(&d).is_empty());
}

#[test]
fn missing_request_file_is_io_error_74() {
    let d = workdir("missing-request");
    let o = run(&[
        "verify",
        "--request",
        s(&d.join("nope.cbor")),
        "--transcript-out",
        s(&d.join("tB.bin")),
    ]);
    assert_eq!(o.status.code(), Some(74));
    let j = stdout_json(&o);
    assert_eq!(j["status"], "io_error");
    assert_eq!(j["code"], "request_read_failed");
    assert!(entries(&d).is_empty());
}

#[test]
fn existing_transcript_is_rejected_without_force() {
    let d = workdir("exists-transcript");
    let tb = d.join("tB.bin");
    fs::write(&tb, b"precious").unwrap();
    let o = run(&[
        "verify",
        "--request",
        s(&fixture(VERIFIED_REQ)),
        "--transcript-out",
        s(&tb),
    ]);
    assert_eq!(o.status.code(), Some(73));
    assert_eq!(fs::read(&tb).unwrap(), b"precious");
    let j = stdout_json(&o);
    assert_eq!(j["status"], "io_error");
    assert_eq!(j["code"], "output_exists");
    assert_eq!(entries(&d), ["tB.bin"]);
}

#[test]
fn existing_result_file_is_rejected_without_force_and_nothing_is_written() {
    let d = workdir("exists-result");
    let (tb, res) = (d.join("tB.bin"), d.join("result.json"));
    fs::write(&res, b"old").unwrap();
    let o = run(&[
        "verify",
        "--request",
        s(&fixture(VERIFIED_REQ)),
        "--transcript-out",
        s(&tb),
        "--result-out",
        s(&res),
    ]);
    assert_eq!(o.status.code(), Some(73));
    assert!(o.stdout.is_empty());
    assert_eq!(fs::read(&res).unwrap(), b"old");
    assert!(
        fs::metadata(&tb).is_err(),
        "preflight must reject before any transcript is written"
    );
}

#[test]
fn existing_outputs_are_rejected_even_for_invalid_results() {
    let d = workdir("exists-invalid");
    let tb = d.join("tB.bin");
    fs::write(&tb, b"precious").unwrap();
    let o = run(&[
        "verify",
        "--request",
        s(&fixture(INVALID_REQ)),
        "--transcript-out",
        s(&tb),
    ]);
    assert_eq!(
        o.status.code(),
        Some(73),
        "preflight happens before semantic execution"
    );
    assert_eq!(fs::read(&tb).unwrap(), b"precious");
}

#[test]
fn force_replaces_only_the_named_outputs() {
    let d = workdir("force");
    let (tb, res, other) = (d.join("tB.bin"), d.join("result.json"), d.join("other.txt"));
    fs::write(&tb, b"old transcript").unwrap();
    fs::write(&res, b"old result").unwrap();
    fs::write(&other, b"untouched").unwrap();
    let o = run(&[
        "verify",
        "--request",
        s(&fixture(VERIFIED_REQ)),
        "--transcript-out",
        s(&tb),
        "--result-out",
        s(&res),
        "--force",
    ]);
    assert_eq!(o.status.code(), Some(0));
    assert_eq!(sha256_hex(&fs::read(&tb).unwrap()), VERIFIED_TB_SHA);
    let j: Value = serde_json::from_slice(&fs::read(&res).unwrap()).unwrap();
    assert_eq!(j["status"], "verified");
    assert_eq!(fs::read(&other).unwrap(), b"untouched");
    assert_eq!(entries(&d), ["other.txt", "result.json", "tB.bin"]);
}

#[test]
fn force_does_not_create_a_transcript_for_invalid_and_keeps_the_old_one() {
    let d = workdir("force-invalid");
    let tb = d.join("tB.bin");
    fs::write(&tb, b"old transcript").unwrap();
    let o = run(&[
        "verify",
        "--request",
        s(&fixture(INVALID_REQ)),
        "--transcript-out",
        s(&tb),
        "--force",
    ]);
    assert_eq!(o.status.code(), Some(2));
    assert_eq!(
        fs::read(&tb).unwrap(),
        b"old transcript",
        "an Invalid result never touches the transcript path"
    );
}

#[test]
fn missing_output_directory_is_exit_73_and_never_verified() {
    let d = workdir("no-dir");
    let tb = d.join("no-such-dir").join("tB.bin");
    let o = run(&[
        "verify",
        "--request",
        s(&fixture(VERIFIED_REQ)),
        "--transcript-out",
        s(&tb),
    ]);
    assert_eq!(o.status.code(), Some(73));
    let j = stdout_json(&o);
    assert_eq!(j["status"], "io_error");
    assert_eq!(j["code"], "output_cannot_be_created");
    assert!(entries(&d).is_empty());
}

#[test]
fn transcript_path_that_is_a_directory_is_rejected_even_with_force() {
    let d = workdir("dir-target");
    let tb = d.join("tB.bin");
    fs::create_dir(&tb).unwrap();
    let o = run(&[
        "verify",
        "--request",
        s(&fixture(VERIFIED_REQ)),
        "--transcript-out",
        s(&tb),
        "--force",
    ]);
    assert_eq!(o.status.code(), Some(73));
    assert!(fs::metadata(&tb).unwrap().is_dir());
}

#[cfg(target_os = "linux")]
#[test]
fn undeliverable_result_removes_the_new_transcript_and_exits_74() {
    let d = workdir("dev-full");
    let tb = d.join("tB.bin");
    let full = fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let o = bin()
        .args([
            "verify",
            "--request",
            s(&fixture(VERIFIED_REQ)),
            "--transcript-out",
            s(&tb),
        ])
        .stdout(full)
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(74));
    assert!(
        fs::metadata(&tb).is_err(),
        "transcript created by this run must not be left behind"
    );
    assert!(entries(&d).is_empty());
}

// ------------------------------------------------------------------------------------------ broken result output

/// Run with stdout pointing at `/dev/full`, so that every write of the result JSON fails.
#[cfg(target_os = "linux")]
fn run_with_full_stdout(args: &[&str]) -> Output {
    let full = fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    bin()
        .args(args)
        .stdout(full)
        .stderr(Stdio::piped())
        .output()
        .unwrap()
}

#[cfg(target_os = "linux")]
#[test]
fn malformed_request_with_broken_stdout_exits_74_and_explains_both_failures() {
    let d = workdir("broken-stdout-transport");
    let req = d.join("req.cbor");
    fs::write(&req, b"junk").unwrap();
    let tb = d.join("tB.bin");
    let args = ["verify", "--request", s(&req), "--transcript-out", s(&tb)];

    // Control: the same request with a working stdout is a transport error, exit 65.
    assert_eq!(run(&args).status.code(), Some(65));

    let o = run_with_full_stdout(&args);
    assert_eq!(
        o.status.code(),
        Some(74),
        "result-delivery failure must win over the earlier 65"
    );
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(
        err.contains("request decode failed"),
        "original failure missing: {err}"
    );
    assert!(
        err.contains("could not deliver the result JSON"),
        "delivery failure missing: {err}"
    );
    assert_eq!(entries(&d), ["req.cbor"]);
}

#[cfg(target_os = "linux")]
#[test]
fn transcript_preflight_error_with_broken_stdout_exits_74() {
    let d = workdir("broken-stdout-preflight");
    let tb = d.join("tB.bin");
    fs::write(&tb, b"precious").unwrap();
    let req = fixture(VERIFIED_REQ);
    let args = ["verify", "--request", s(&req), "--transcript-out", s(&tb)];

    // Control: working stdout reports the preflight failure, exit 73.
    assert_eq!(run(&args).status.code(), Some(73));

    let o = run_with_full_stdout(&args);
    assert_eq!(
        o.status.code(),
        Some(74),
        "result-output failure code wins over 73"
    );
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(
        err.contains("already exists"),
        "original failure missing: {err}"
    );
    assert!(
        err.contains("could not deliver the result JSON"),
        "delivery failure missing: {err}"
    );
    assert_eq!(fs::read(&tb).unwrap(), b"precious");
}

#[cfg(target_os = "linux")]
#[test]
fn missing_request_with_broken_stdout_exits_74() {
    let d = workdir("broken-stdout-missing-request");
    let o = run_with_full_stdout(&[
        "verify",
        "--request",
        s(&d.join("nope.cbor")),
        "--transcript-out",
        s(&d.join("tB.bin")),
    ]);
    assert_eq!(o.status.code(), Some(74));
    assert!(String::from_utf8_lossy(&o.stderr).contains("could not deliver the result JSON"));
}

#[cfg(target_os = "linux")]
#[test]
fn invalid_with_broken_stdout_exits_74_not_2() {
    let d = workdir("broken-stdout-invalid");
    let o = run_with_full_stdout(&[
        "verify",
        "--request",
        s(&fixture(INVALID_REQ)),
        "--transcript-out",
        s(&d.join("tB.bin")),
    ]);
    assert_eq!(o.status.code(), Some(74));
    assert!(entries(&d).is_empty());
}

// ------------------------------------------------------------------------------------------ path conflicts

#[test]
fn conflicting_paths_are_usage_errors_and_touch_nothing() {
    let d = workdir("conflicts");
    let req = d.join("req.cbor");
    fs::copy(fixture(VERIFIED_REQ), &req).unwrap();
    let same = d.join("same.out");
    let alias = d.join(".").join("same.out");

    // transcript == result
    let o = run(&[
        "verify",
        "--request",
        s(&req),
        "--transcript-out",
        s(&same),
        "--result-out",
        s(&alias),
    ]);
    assert_eq!(o.status.code(), Some(64));
    // request == transcript, with and without --force
    for extra in [&[][..], &["--force"][..]] {
        let mut args = vec!["verify", "--request", s(&req), "--transcript-out", s(&req)];
        args.extend_from_slice(extra);
        let o = run(&args);
        assert_eq!(o.status.code(), Some(64));
        assert!(o.stdout.is_empty());
    }
    // request == result
    let o = run(&[
        "verify",
        "--request",
        s(&req),
        "--transcript-out",
        s(&same),
        "--result-out",
        s(&req),
        "--force",
    ]);
    assert_eq!(o.status.code(), Some(64));

    assert_eq!(
        sha256_hex(&fs::read(&req).unwrap()),
        VERIFIED_REQ_SHA,
        "request file untouched"
    );
    assert_eq!(entries(&d), ["req.cbor"]);
}
