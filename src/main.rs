//! `p10-replay`: a thin command-line adapter around the pinned P10 S2b replay verifier.
//!
//! The semantic path is exactly `decode_request` followed by `replay_p0_p9`. Everything else in this crate
//! is argument parsing, file handling and rendering of already-returned core values.

mod cli;
mod json;
mod output;
mod report;

use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::panic::{self, AssertUnwindSafe};
use std::path::Path;

use p10_replay::finalize::ReplayResult;

use cli::{Command, RequestSource, ResultSink, VerifyArgs};
use json::Json;
use output::OutError;

const EXIT_VERIFIED: u8 = 0;
const EXIT_INVALID: u8 = 2;
const EXIT_UNAVAILABLE: u8 = 3;
const EXIT_USAGE: u8 = 64;
const EXIT_TRANSPORT: u8 = 65;
const EXIT_INTERNAL: u8 = 70;
const EXIT_OTHER_IO: u8 = 74;

/// Must match `Cargo.toml` and `Cargo.lock`; checked by a test.
const CORE_TAG: &str = "v0.1.1-s2b-public";
const CORE_COMMIT: &str = "8e89f9a4192bdd0ab280556701dc595eb55539d2";

const HELP: &str = "\
p10-replay: command-line adapter for the P10 S2b replay verifier

USAGE:
    p10-replay verify --request <request.cbor|-> --transcript-out <tB.bin>
                      [--result-out <result.json|->] [--force]
    p10-replay --help | --version

Exit code 0 means S2b transcript faithfulness only. It is not an S2a ACCEPT
and not a final P10 acceptance.

Run 'p10-replay verify --help' for details.
";

const VERIFY_HELP: &str = "\
p10-replay verify: replay one request with the pinned S2b verifier

USAGE:
    p10-replay verify --request <request.cbor|-> --transcript-out <tB.bin>
                      [--result-out <result.json|->] [--force]

OPTIONS:
    --request <path|->        exact request bytes; '-' reads stdin
    --transcript-out <path>   where the exact tB bytes go on Verified (required, never stdout)
    --result-out <path|->     CLI result JSON; default '-' (stdout)
    --force                   allow replacing the explicitly named output files
    -h, --help                print this help to stderr

EXIT CODES:
     0  Verified (S2b transcript faithfulness only; not S2a, not a P10 verdict)
     2  Invalid
     3  Unavailable
    64  usage error
    65  request transport/decode error
    73  output cannot be created, or exists and --force was not given
    74  other input/output error
    70  unexpected internal failure

No transcript file is created unless the result is Verified.
";

fn say_err(msg: &str) {
    let _ = writeln!(io::stderr(), "p10-replay: {msg}");
}

fn main() {
    // Panics must not reach stdout and must map to exit 70; the default hook already writes to stderr.
    panic::set_hook(Box::new(|info| {
        let _ = writeln!(io::stderr(), "p10-replay: internal error: {info}");
    }));
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let code = match panic::catch_unwind(AssertUnwindSafe(|| run(args))) {
        Ok(c) => c,
        Err(_) => EXIT_INTERNAL,
    };
    std::process::exit(i32::from(code));
}

fn run(args: Vec<OsString>) -> u8 {
    match cli::parse(args) {
        Err(e) => {
            say_err(&format!("{}\nTry 'p10-replay --help'.", e.0));
            EXIT_USAGE
        }
        Ok(Command::Help) => {
            let _ = io::stdout().write_all(HELP.as_bytes());
            0
        }
        Ok(Command::Version) => {
            let out = format!(
                "p10-replay {}\ncore: p10_replay {CORE_TAG} ({CORE_COMMIT})\n",
                env!("CARGO_PKG_VERSION")
            );
            let _ = io::stdout().write_all(out.as_bytes());
            0
        }
        Ok(Command::VerifyHelp) => {
            let _ = io::stderr().write_all(VERIFY_HELP.as_bytes());
            0
        }
        Ok(Command::Verify(args)) => verify(&args),
    }
}

/// Deliver the result JSON to its destination.
fn emit(json: &Json, sink: &ResultSink, force: bool) -> Result<(), OutError> {
    let line = json.to_line();
    match sink {
        ResultSink::Stdout => {
            let mut out = io::stdout().lock();
            out.write_all(line.as_bytes())
                .and_then(|()| out.flush())
                .map_err(|e| OutError::Io(format!("cannot write result to stdout: {e}")))
        }
        ResultSink::File(path) => output::stage(path, line.as_bytes(), force)?
            .commit()
            .map(|_| ()),
    }
}

/// Emit a non-semantic failure result (transport or I/O) and return its exit code. If the result itself cannot
/// be delivered, the final problem is output I/O: both failures are reported on stderr and the delivery
/// failure's exit code wins.
fn emit_and_exit(json: &Json, sink: &ResultSink, force: bool, code: u8, message: &str) -> u8 {
    say_err(message);
    match emit(json, sink, force) {
        Ok(()) => code,
        Err(e) => {
            say_err(&format!(
                "also could not deliver the result JSON: {}",
                e.message()
            ));
            e.exit_code()
        }
    }
}

fn read_request(src: &RequestSource) -> io::Result<Vec<u8>> {
    match src {
        RequestSource::Stdin => {
            let mut buf = Vec::new();
            io::stdin().lock().read_to_end(&mut buf)?;
            Ok(buf)
        }
        RequestSource::File(p) => std::fs::read(p),
    }
}

fn verify(args: &VerifyArgs) -> u8 {
    let sink = &args.result_out;
    let force = args.force;

    // 1. Argument combinations and output preflight, before any semantic work.
    if let Err(e) = output::check_conflicts(args) {
        say_err(&format!("{}\nTry 'p10-replay verify --help'.", e.0));
        return EXIT_USAGE;
    }
    if let ResultSink::File(p) = sink {
        if let Err(e) = output::preflight(p, force) {
            // The result file itself is unusable, so there is nowhere to report a JSON result.
            say_err(e.message());
            return e.exit_code();
        }
    }
    if let Err(e) = output::preflight(&args.transcript_out, force) {
        let json = report::io_error(e.code(), e.message());
        return emit_and_exit(&json, sink, force, e.exit_code(), e.message());
    }

    // 2. Read the exact request bytes.
    let raw = match read_request(&args.request) {
        Ok(b) => b,
        Err(e) => {
            let what = match &args.request {
                RequestSource::Stdin => "stdin".to_owned(),
                RequestSource::File(p) => format!("'{}'", p.display()),
            };
            let msg = format!("cannot read request from {what}: {e}");
            let json = report::io_error("request_read_failed", &msg);
            return emit_and_exit(&json, sink, force, EXIT_OTHER_IO, &msg);
        }
    };

    // 3. The core call path. Nothing else is called, and nothing inspects diagnostics.
    let request = match p10_replay::request::decode_request(&raw) {
        Ok(r) => r,
        Err(e) => {
            let msg = e.to_string();
            let json = report::transport_error("request_decode_failed", &msg);
            return emit_and_exit(
                &json,
                sink,
                force,
                EXIT_TRANSPORT,
                &format!("request decode failed: {msg}"),
            );
        }
    };
    let result = p10_replay::finalize::replay_p0_p9(&request);

    // 4. Render and write.
    match &result {
        ReplayResult::Invalid {
            reason,
            diagnostics,
        } => finish_without_transcript(
            &report::failed("invalid", reason, diagnostics),
            sink,
            force,
            EXIT_INVALID,
        ),
        ReplayResult::Unavailable {
            reason,
            diagnostics,
        } => finish_without_transcript(
            &report::failed("unavailable", reason, diagnostics),
            sink,
            force,
            EXIT_UNAVAILABLE,
        ),
        ReplayResult::Verified {
            transcript_bytes,
            replay_evidence,
        } => finish_verified(
            transcript_bytes,
            &report::verified(transcript_bytes.len(), replay_evidence),
            args,
        ),
    }
}

fn finish_without_transcript(json: &Json, sink: &ResultSink, force: bool, code: u8) -> u8 {
    match emit(json, sink, force) {
        Ok(()) => code,
        Err(e) => {
            say_err(e.message());
            e.exit_code()
        }
    }
}

fn finish_verified(transcript: &[u8], verified_json: &Json, args: &VerifyArgs) -> u8 {
    let (sink, force) = (&args.result_out, args.force);

    let created = match output::stage(&args.transcript_out, transcript, force)
        .and_then(output::Staged::commit)
    {
        Ok(created) => created,
        Err(e) => {
            // Never report `verified` when the transcript is not on disk.
            let json = report::io_error("transcript_write_failed", e.message());
            return emit_and_exit(&json, sink, force, e.exit_code(), e.message());
        }
    };

    match emit(verified_json, sink, force) {
        Ok(()) => EXIT_VERIFIED,
        Err(e) => {
            say_err(e.message());
            if created {
                remove_new_transcript(&args.transcript_out);
            }
            e.exit_code()
        }
    }
}

/// The result could not be delivered, so the transcript just created is not left behind unannounced.
fn remove_new_transcript(path: &Path) {
    match std::fs::remove_file(path) {
        Ok(()) => say_err(&format!(
            "removed newly created transcript '{}'",
            path.display()
        )),
        Err(e) => say_err(&format!(
            "could not remove transcript '{}': {e}",
            path.display()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_pin_constants_match_manifest_and_lock() {
        let manifest = include_str!("../Cargo.toml");
        assert!(manifest.contains(&format!("tag = \"{CORE_TAG}\"")));
        let lock = include_str!("../Cargo.lock");
        let want = format!(
            "git+https://github.com/VolMax-Studio/p10-replay-verifier?tag={CORE_TAG}#{CORE_COMMIT}"
        );
        assert!(lock.contains(&want), "Cargo.lock does not pin {want}");
    }
}
