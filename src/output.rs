//! Output-file safety: preflight, conflict detection, and temp-file-plus-rename writes.
//!
//! Temporary files live in the destination's own directory, are created with `create_new`, and are removed
//! on every failure path (including unwinding) by `Staged::drop`.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use crate::cli::{RequestSource, ResultSink, UsageError, VerifyArgs};

/// Why an output could not be produced. `Exists` and `Create` map to exit 73, `Io` to exit 74.
#[derive(Debug)]
pub enum OutError {
    Exists(String),
    Create(String),
    Io(String),
}

impl OutError {
    pub fn code(&self) -> &'static str {
        match self {
            OutError::Exists(_) => "output_exists",
            OutError::Create(_) => "output_cannot_be_created",
            OutError::Io(_) => "output_write_failed",
        }
    }

    pub fn message(&self) -> &str {
        match self {
            OutError::Exists(m) | OutError::Create(m) | OutError::Io(m) => m,
        }
    }

    pub fn exit_code(&self) -> u8 {
        match self {
            OutError::Exists(_) | OutError::Create(_) => 73,
            OutError::Io(_) => 74,
        }
    }
}

fn parent_of(path: &Path) -> &Path {
    match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    }
}

/// Best-effort identity of a path that may not exist yet: canonical parent plus file name.
fn identity(path: &Path) -> PathBuf {
    if let Ok(c) = fs::canonicalize(path) {
        return c;
    }
    match (fs::canonicalize(parent_of(path)), path.file_name()) {
        (Ok(dir), Some(name)) => dir.join(name),
        _ => std::env::current_dir()
            .map(|d| d.join(path))
            .unwrap_or_else(|_| path.to_path_buf()),
    }
}

/// Reject argument combinations in which two roles would name the same file.
pub fn check_conflicts(args: &VerifyArgs) -> Result<(), UsageError> {
    let transcript = identity(&args.transcript_out);
    if let ResultSink::File(r) = &args.result_out {
        if identity(r) == transcript {
            return Err(UsageError(
                "'--result-out' and '--transcript-out' name the same file".into(),
            ));
        }
    }
    if let RequestSource::File(req) = &args.request {
        let req = identity(req);
        if req == transcript {
            return Err(UsageError(
                "'--request' and '--transcript-out' name the same file".into(),
            ));
        }
        if let ResultSink::File(r) = &args.result_out {
            if identity(r) == req {
                return Err(UsageError(
                    "'--request' and '--result-out' name the same file".into(),
                ));
            }
        }
    }
    Ok(())
}

/// Structural check of one explicitly named output file. Creates and truncates nothing.
pub fn preflight(dest: &Path, force: bool) -> Result<(), OutError> {
    let shown = dest.display();
    if dest.file_name().is_none() {
        return Err(OutError::Create(format!("'{shown}' does not name a file")));
    }
    let dir = parent_of(dest);
    match fs::metadata(dir) {
        Ok(m) if m.is_dir() => {}
        Ok(_) => {
            return Err(OutError::Create(format!(
                "parent of '{shown}' is not a directory"
            )))
        }
        Err(e) => {
            return Err(OutError::Create(format!(
                "parent directory of '{shown}' is not usable: {e}"
            )))
        }
    }
    match fs::symlink_metadata(dest) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(OutError::Io(format!("cannot inspect '{shown}': {e}"))),
        Ok(_) if !force => Err(OutError::Exists(format!(
            "'{shown}' already exists (use --force to replace it)"
        ))),
        Ok(m) if m.is_file() || m.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(OutError::Create(format!(
            "'{shown}' exists and is not a regular file"
        ))),
    }
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A fully written temporary file waiting to be moved into place.
pub struct Staged {
    tmp: PathBuf,
    dest: PathBuf,
    force: bool,
    armed: bool,
}

/// Write `bytes` to a fresh temporary file next to `dest`.
pub fn stage(dest: &Path, bytes: &[u8], force: bool) -> Result<Staged, OutError> {
    let dir = parent_of(dest);
    let name = dest
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut last = None;
    for _ in 0..16 {
        let tmp = dir.join(format!(
            ".{name}.p10-replay-tmp-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        match OpenOptions::new().write(true).create_new(true).open(&tmp) {
            Ok(mut f) => {
                // From here on, dropping `staged` (error or unwind) removes the temporary file.
                let staged = Staged {
                    tmp,
                    dest: dest.to_path_buf(),
                    force,
                    armed: true,
                };
                f.write_all(bytes)
                    .and_then(|()| f.sync_all())
                    .map_err(|e| {
                        OutError::Io(format!("writing '{}' failed: {e}", dest.display()))
                    })?;
                return Ok(staged);
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => last = Some(e),
            Err(e) => {
                return Err(OutError::Create(format!(
                    "cannot create a temporary file for '{}': {e}",
                    dest.display()
                )))
            }
        }
    }
    let why = last.map(|e| e.to_string()).unwrap_or_default();
    Err(OutError::Create(format!(
        "cannot create a temporary file for '{}': {why}",
        dest.display()
    )))
}

impl Staged {
    /// Move into place. Without `--force` the destination must not exist (atomic hard-link, never replaces);
    /// with `--force` an existing file is atomically replaced. Returns whether the destination is new.
    pub fn commit(mut self) -> Result<bool, OutError> {
        let shown = self.dest.display().to_string();
        let existed = fs::symlink_metadata(&self.dest).is_ok();
        if self.force {
            fs::rename(&self.tmp, &self.dest)
                .map_err(|e| OutError::Io(format!("cannot move result into '{shown}': {e}")))?;
            self.armed = false;
        } else {
            match fs::hard_link(&self.tmp, &self.dest) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    return Err(OutError::Exists(format!(
                        "'{shown}' already exists (use --force to replace it)"
                    )));
                }
                Err(e) => return Err(OutError::Io(format!("cannot create '{shown}': {e}"))),
            }
            // `armed` stays set: drop removes the now-redundant temporary name.
        }
        Ok(!existed)
    }
}

impl Drop for Staged {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_file(&self.tmp);
        }
    }
}
