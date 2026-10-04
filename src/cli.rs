//! Small explicit argument parser. No framework; no globbing, no `--opt=value`, no short options except
//! `-h` / `-V`.

use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub struct UsageError(pub String);

#[derive(Debug, PartialEq, Eq)]
pub enum RequestSource {
    Stdin,
    File(PathBuf),
}

#[derive(Debug, PartialEq, Eq)]
pub enum ResultSink {
    Stdout,
    File(PathBuf),
}

#[derive(Debug, PartialEq, Eq)]
pub struct VerifyArgs {
    pub request: RequestSource,
    pub transcript_out: PathBuf,
    pub result_out: ResultSink,
    pub force: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    /// Top-level `--help`: printed to stdout.
    Help,
    /// `--version`: printed to stdout.
    Version,
    /// `verify --help`: printed to stderr.
    VerifyHelp,
    Verify(VerifyArgs),
}

fn usage(msg: impl Into<String>) -> UsageError {
    UsageError(msg.into())
}

/// Parse the arguments after the program name.
pub fn parse(args: Vec<OsString>) -> Result<Command, UsageError> {
    let mut it = args.into_iter();
    let Some(first) = it.next() else {
        return Err(usage("missing command (expected 'verify')"));
    };
    match first.to_str() {
        Some("--help" | "-h") => no_more(it, Command::Help),
        Some("--version" | "-V") => no_more(it, Command::Version),
        Some("verify") => parse_verify(it.collect()),
        Some(other) if other.starts_with('-') && other != "-" => {
            Err(usage(format!("unknown option '{other}'")))
        }
        Some(other) => Err(usage(format!(
            "unknown command '{other}' (expected 'verify')"
        ))),
        None => Err(usage("unknown command (argument is not valid UTF-8)")),
    }
}

fn no_more(mut it: impl Iterator<Item = OsString>, cmd: Command) -> Result<Command, UsageError> {
    match it.next() {
        None => Ok(cmd),
        Some(extra) => Err(usage(format!(
            "unexpected argument '{}'",
            extra.to_string_lossy()
        ))),
    }
}

fn parse_verify(args: Vec<OsString>) -> Result<Command, UsageError> {
    let mut request: Option<OsString> = None;
    let mut transcript_out: Option<OsString> = None;
    let mut result_out: Option<OsString> = None;
    let mut force = false;

    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        let Some(name) = arg.to_str() else {
            return Err(usage(format!(
                "unexpected argument '{}'",
                arg.to_string_lossy()
            )));
        };
        let slot = match name {
            "--help" | "-h" => return Ok(Command::VerifyHelp),
            "--force" => {
                if force {
                    return Err(usage("duplicate option '--force'"));
                }
                force = true;
                continue;
            }
            "--request" => &mut request,
            "--transcript-out" => &mut transcript_out,
            "--result-out" => &mut result_out,
            other if other.starts_with('-') && other != "-" => {
                return Err(usage(format!("unknown option '{other}'")))
            }
            other => return Err(usage(format!("unexpected argument '{other}'"))),
        };
        if slot.is_some() {
            return Err(usage(format!("duplicate option '{name}'")));
        }
        // A value that itself looks like an option is treated as a missing value; use './--name' for such files.
        match it.next() {
            Some(v) if !v.to_str().is_some_and(|s| s.starts_with("--")) => *slot = Some(v),
            _ => return Err(usage(format!("option '{name}' requires a value"))),
        }
    }

    let request = request.ok_or_else(|| usage("missing required option '--request'"))?;
    let transcript_out =
        transcript_out.ok_or_else(|| usage("missing required option '--transcript-out'"))?;
    if transcript_out.is_empty()
        || request.is_empty()
        || result_out.as_ref().is_some_and(|v| v.is_empty())
    {
        return Err(usage("empty path argument"));
    }
    if transcript_out == "-" {
        return Err(usage(
            "'--transcript-out -' is not allowed: transcript bytes are never written to stdout",
        ));
    }

    Ok(Command::Verify(VerifyArgs {
        request: if request == "-" {
            RequestSource::Stdin
        } else {
            RequestSource::File(request.into())
        },
        transcript_out: transcript_out.into(),
        result_out: match result_out {
            Some(v) if v != "-" => ResultSink::File(v.into()),
            _ => ResultSink::Stdout,
        },
        force,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Result<Command, UsageError> {
        parse(args.iter().map(OsString::from).collect())
    }

    #[test]
    fn top_level() {
        assert_eq!(p(&["--help"]), Ok(Command::Help));
        assert_eq!(p(&["--version"]), Ok(Command::Version));
        assert_eq!(p(&["verify", "--help"]), Ok(Command::VerifyHelp));
        assert!(p(&[]).is_err());
        assert!(p(&["--help", "x"]).is_err());
        assert!(p(&["nope"]).is_err());
    }

    #[test]
    fn verify_defaults_and_stdin() {
        let Ok(Command::Verify(a)) = p(&["verify", "--request", "-", "--transcript-out", "t.bin"])
        else {
            panic!()
        };
        assert_eq!(a.request, RequestSource::Stdin);
        assert_eq!(a.result_out, ResultSink::Stdout);
        assert!(!a.force);
    }

    #[test]
    fn verify_rejections() {
        assert!(p(&["verify", "--transcript-out", "t"]).is_err());
        assert!(p(&["verify", "--request", "r"]).is_err());
        assert!(p(&[
            "verify",
            "--request",
            "r",
            "--request",
            "r2",
            "--transcript-out",
            "t"
        ])
        .is_err());
        assert!(p(&[
            "verify",
            "--request",
            "r",
            "--transcript-out",
            "t",
            "--force",
            "--force"
        ])
        .is_err());
        assert!(p(&[
            "verify",
            "--request",
            "r",
            "--transcript-out",
            "t",
            "--bogus"
        ])
        .is_err());
        assert!(p(&["verify", "--request", "r", "--transcript-out", "t", "extra"]).is_err());
        assert!(p(&["verify", "--request", "r", "--transcript-out"]).is_err());
        assert!(p(&["verify", "--request", "--transcript-out", "t"]).is_err());
        assert!(p(&["verify", "--request", "r", "--transcript-out", "-"]).is_err());
    }
}
