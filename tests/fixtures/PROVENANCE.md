# Fixture provenance

These files are **test data**. They are not a second implementation of S2b and not an acceptance corpus. The CLI never interprets them; it hands the
bytes to the pinned core, and the tests compare the CLI's behavior with what the S2b tag already records for these vectors.

| Item | Value |
|---|---|
| Source repository | https://github.com/VolMax-Studio/p10-replay-verifier |
| Source tag | `v0.1.1-s2b-public` (annotated tag object `d98179fb060ddf94f138bd5bb9720afdcb58ddcf`) |
| Tag peel commit | `8e89f9a4192bdd0ab280556701dc595eb55539d2` |
| Copy method | `cp` from a checkout of that tag; bytes unchanged |

| Copied file | Original path in the source tag | SHA-256 (copied file) | Expected behavior |
|---|---|---|---|
| `verified/request.cbor` | `vectors/SPS/SPS-5-predicate-null/request.cbor` | `b22b7e70c6750d19c2278275b49d8eb9092832e2d833df2ba08bfb4fdacbbae9` | `Verified`; `tB` SHA-256 `731452bded7d86b74985dfd449f8ece8c2fc600cec5fea8d92d873cd60a6c865` |
| `invalid/request.cbor` | `vectors/SPS/SPS-17-nfc-subject/request.cbor` | `807840396ae229047f59a053a716fa6857d0d6abe109725faaca446848528222` | `Invalid(request, TargetKindMismatch)` |
| `unavailable/request.cbor` | `vectors/R/R12a/request.cbor` | `4f3596aafc0515459b91e766e3c7d9383797de2c9e7a5d35e650ff3cad2e7300` | `Unavailable(material, MissingIndex { first_missing: 4 })` |

`SHA256SUMS` (`sha256sum -c` format, paths relative to this directory) records the three request hashes. The expected `tB` hash and the expected
results above are the values given in the CLI authorization; they are asserted by `tests/cli.rs` against the CLI's actual output.

The tag is annotated and was created by its maintainer. This repository did not verify the tag signature (no signer key is configured in the build
environment); the commit identity above was resolved with `git ls-remote` and from the checked-out tag.
