# p10-replay-cli

A small, auditable command-line wrapper around the **P10 S2b replay verifier**.

`p10-replay verify` reads one `ReplayRequestV0` file, runs the pinned S2b verifier on it, writes the exact transcript bytes (`tB`) when, and only when,
the result is `Verified`, and prints a small CLI-owned JSON result. It is an **adapter only**: it does not reimplement, reinterpret or modify any S2b
semantics.

## Pinned core

```toml
p10_replay = { git = "https://github.com/VolMax-Studio/p10-replay-verifier", tag = "v0.1.1-s2b-public" }
```

`Cargo.lock` resolves it to commit `8e89f9a4192bdd0ab280556701dc595eb55539d2` (the peeled commit of the tag). The only other direct dependency is
`serde_json =1.0.132`, used for string escaping in the CLI's own JSON output.

The whole semantic path is two calls:

```rust
let request = p10_replay::request::decode_request(&raw_bytes)?;
let result  = p10_replay::finalize::replay_p0_p9(&request);
```

## Install from source

```sh
git clone https://github.com/VolMax-Studio/p10-replay-cli
cd p10-replay-cli
cargo build --release --locked      # binary: target/release/p10-replay
```

Not published to crates.io. Minimum Rust version: 1.82; `rust-toolchain.toml` pins 1.82.0 and `Cargo.lock` uses the same versions as the S2b public tag for every shared dependency.

## Usage

```sh
p10-replay verify \
  --request <request.cbor|-> \
  --transcript-out <tB.bin> \
  [--result-out <result.json|->] \
  [--force]
```

```sh
# result JSON on stdout, transcript in a file
p10-replay verify --request request.cbor --transcript-out tB.bin

# request from stdin, result JSON in a file, replace earlier outputs
cat request.cbor | p10-replay verify --request - --transcript-out tB.bin --result-out result.json --force
```

* `--transcript-out` is mandatory. Transcript bytes are never written to stdout.
* `--result-out` defaults to `-` (stdout). With `-`, stdout carries only the result JSON, ending in exactly one newline.
* Existing output files are never overwritten unless `--force` is given, and `--force` only affects the files you name.
* A transcript file is created only for `Verified`. Nothing is created for Invalid, Unavailable, decode, usage or I/O failures.
* Help (`--help`, `--version`) goes to stdout; `verify --help` and all operational messages go to stderr.

## Exit codes

| code | meaning |
|---|---|
| 0 | `ReplayResult::Verified` |
| 2 | `ReplayResult::Invalid` |
| 3 | `ReplayResult::Unavailable` |
| 64 | command-line usage error |
| 65 | request transport/decode error |
| 73 | requested output cannot be created, or would overwrite without `--force` |
| 74 | other input/output error |
| 70 | unexpected internal CLI failure |

If the result JSON itself cannot be delivered (for example stdout is broken), the exit code is the output failure's (73/74), even if an earlier
failure such as 65 was already established; stderr reports both.

## What `Verified` means

Exit code 0 and `"status": "verified"` mean **S2b transcript faithfulness and nothing more**: the pinned verifier replayed the supplied request
(P0 to P9) and produced the transcript `tB` that was written, whose SHA-256 is reported.

It does **not** mean S2a `ACCEPT`, and it is **not** a final P10 verdict or acceptance. Nothing after S2b has been run or implied.

## What this is not

* Not S2a, and not S3. It does not run S2a or Lean and performs no post-S2a binding.
* Not a log client: it fetches nothing, resolves no keys, and uses no network. The request must already contain everything.
* Not a final policy engine: it issues no accept/reject decision, it forwards the core's result.
* It does not parse CBOR, COSE, JSON, Receipts, Statements or `tB` itself, and it does not look at diagnostics to choose an outcome.

## Result schema

`P10ReplayCliResultV0` is CLI-owned and non-normative; see [`docs/RESULT_SCHEMA.md`](docs/RESULT_SCHEMA.md). There is deliberately no
`--evidence-out`: the core defines no wire encoding for `ReplayEvidence`, so the CLI reports a fixed summary plus the exact `tB`.

## Tests

```sh
cargo test --locked --offline
```

Fixtures are three requests copied from the S2b public tag; see [`tests/fixtures/PROVENANCE.md`](tests/fixtures/PROVENANCE.md).

## Links

* S2b verifier: https://github.com/VolMax-Studio/p10-replay-verifier
* Public release used: https://github.com/VolMax-Studio/p10-replay-verifier/releases/tag/v0.1.1-s2b-public

## License

MIT OR Apache-2.0 (see [`LICENSE-MIT`](LICENSE-MIT), [`LICENSE-APACHE`](LICENSE-APACHE), [`NOTICE.md`](NOTICE.md)).
Copyright (c) 2026 Ivan Nestorov.
