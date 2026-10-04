# Notice

Developed and maintained by Ivan Nestorov through
VolMax Studio Lab d.o.o., Serbia.

Copyright (c) 2026 Ivan Nestorov

## License

`p10-replay-cli` is licensed under **MIT OR Apache-2.0**: you may use it under the terms of either [`LICENSE-MIT`](LICENSE-MIT)
or [`LICENSE-APACHE`](LICENSE-APACHE), at your option.

## What this is

An independent command-line adapter for the P10 S2b replay verifier
([`p10-replay-verifier`](https://github.com/VolMax-Studio/p10-replay-verifier), tag `v0.1.1-s2b-public`). It is not affiliated with, sponsored by,
or endorsed by the IETF, the IETF Trust, or the SCITT Working Group.

## IETF material

No IETF specification text and no marked IETF Code Component is copied into this repository. IETF documents remain subject to the applicable
IETF Trust Legal Provisions.

## Third-party software

The pinned S2b verifier and the Rust crates in `Cargo.lock` carry their own permissive licenses and are used under those licenses.

## Test fixtures

`tests/fixtures/` contains three `request.cbor` files copied unchanged from the S2b public tag (see `tests/fixtures/PROVENANCE.md`). They are
covered by the S2b repository's license (MIT OR Apache-2.0) and are test data only.
