# Security policy

## Supported release

Only the current `0.1.x` source on the default branch is supported once it is released. Nothing is published to crates.io (`publish = false`).

## Scope

This repository is an adapter. It contains **no verification logic**: request decoding, replay, result precedence and transcript construction
all belong to the pinned S2b verifier (`v0.1.1-s2b-public`, commit `8e89f9a4192bdd0ab280556701dc595eb55539d2`). Report problems with those semantics to
the S2b repository's [security policy](https://github.com/VolMax-Studio/p10-replay-verifier/blob/v0.1.1-s2b-public/SECURITY.md).

In scope here:

- the CLI reporting a result other than what the core returned (wrong status, exit code, reason mapping, or transcript bytes);
- writing a transcript for a result that is not `Verified`;
- overwriting or truncating a file without `--force`, or touching a file that was not named on the command line;
- writing transcript bytes to stdout, or any output on stdout besides the result JSON;
- path-handling flaws in output creation (temporary files, rename, conflicts).

## Reporting a vulnerability

Please report privately, not as a public issue:

- through GitHub Security Advisories for this repository, when available, or
- by e-mail to `volmax.core@gmail.com`.

Include the commit, the exact command line, the request file (or its SHA-256), and the expected and observed behavior.

## Change policy

A change to the pinned core, to exit codes, or to `P10ReplayCliResultV0` requires a new reviewed release of this CLI.
