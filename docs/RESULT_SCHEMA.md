# `P10ReplayCliResultV0`

A **CLI-owned, non-normative** result document written by `p10-replay verify`. It is not a persisted encoding of `ReplayEvidence` (the S2b core
defines no evidence wire format), not an S2a result and not a P10 verdict.

* One JSON object, UTF-8, compact (no insignificant whitespace), terminated by exactly one `\n`.
* Keys appear in the order shown below. Consumers should not depend on key order.
* `"schema"` is always `"P10ReplayCliResultV0"`. Unknown future fields must be ignored by consumers.
* Usage errors (exit 64) produce **no** JSON, only a message on stderr.
* If the result JSON itself cannot be delivered (unusable `--result-out` file, failing stdout), only stderr and the exit code report it. Both the original failure and the delivery failure are written to stderr, and the exit code is the delivery failure's (73 or 74), not the earlier 65/73/74.

`status` is one of `verified`, `invalid`, `unavailable`, `transport_error`, `io_error`.

| status | meaning | exit |
|---|---|---|
| `verified` | the core returned `ReplayResult::Verified` and the transcript file was written | 0 |
| `invalid` | the core returned `ReplayResult::Invalid` | 2 |
| `unavailable` | the core returned `ReplayResult::Unavailable` | 3 |
| `transport_error` | `decode_request` rejected the bytes; `replay_p0_p9` was never called | 65 |
| `io_error` | an input or output operation failed | 73 / 74 |

## `verified`

```json
{"schema":"P10ReplayCliResultV0","status":"verified",
 "transcript":{"bytes":123,"sha256":"<64 lowercase hex>"},
 "evidence_summary":{"checkpoint_size":0,"root_digest":"<64 hex>","log_identity_digest":"<64 hex>",
                     "statement_count":0,"subject_observation_count":0,"unreadable_row_count":0}}
```

* `transcript.bytes` is the length of the `tB` bytes written; `transcript.sha256` is copied from the core's `ReplayEvidence.transcript_sha256`
  (not recomputed by the CLI).
* `evidence_summary` is a fixed summary of the core's `ReplayEvidence`: `checkpoint_size`, `root_digest`, `log_identity.log_identity_digest`, and the
  lengths of `statement_sha256`, `subject_observations` and `unreadable_rows`. Digests are lowercase hex of the core's 32-byte values.
* The full `ReplayEvidence` is deliberately not serialized.

## `invalid` and `unavailable`

```json
{"schema":"P10ReplayCliResultV0","status":"invalid",
 "primary":{"phase":"p8","severity":"invalid","class":"request","reason":"target_kind_mismatch","parameters":{},"detail":null},
 "diagnostics":[]}
```

`primary` is the core's `reason` failure, and `diagnostics` is the core's `diagnostics` list in the core's order. Each failure has:

| field | type | values |
|---|---|---|
| `phase` | string | `p0` … `p9`, the lowercase codes of the S2b phases P0–P9 |
| `severity` | string | `invalid`, `unavailable` |
| `class` | string | `request`, `replay`, `binding`, `material`, `capability` |
| `reason` | string | see below |
| `parameters` | object | `{}` except for the two parameterized reasons below |
| `detail` | string or null | free text from the core. **Diagnostic only**: not stable, not to be parsed or used to decide anything |

Reasons (core `Reason` variant → code):

`FrozenConfigMalformed` → `frozen_config_malformed` · `KeyMaterialFingerprintMismatch` → `key_material_fingerprint_mismatch` ·
`VerifierCapabilityUnsupported` → `verifier_capability_unsupported` · `ReceiptAbsent` → `receipt_absent` · `TargetStatementAbsent` →
`target_statement_absent` · `ReceiptMalformed` → `receipt_malformed` · `ReceiptTargetBindingFailure` → `receipt_target_binding_failure` ·
`ReceiptTargetAmbiguous` → `receipt_target_ambiguous` · `ReceiptTargetAuthenticationFailure` → `receipt_target_authentication_failure` ·
`ReceiptBadSignature` → `receipt_bad_signature` · `ReceiptVdsUnsupported` → `receipt_vds_unsupported` · `ReceiptKeyUnavailable` →
`receipt_key_unavailable` · `VdsAlgorithmMismatch` → `vds_algorithm_mismatch` · `DuplicateIndex` → `duplicate_index` · `IndexOutOfPrefix` →
`index_out_of_prefix` · `MissingIndex { first_missing }` → `missing_index` with `{"first_missing": <u64>}` · `RequestMalformed` →
`request_malformed` · `TargetStatementMismatch` → `target_statement_mismatch` · `RootMismatch` → `root_mismatch` · `TsIssuerMismatch` →
`ts_issuer_mismatch` · `LogIdentityInconsistent` → `log_identity_inconsistent` · `LeafSpecDigestMismatch` → `leaf_spec_digest_mismatch` ·
`LeafInputDomainViolation` → `leaf_input_domain_violation` · `TargetKindMismatch` → `target_kind_mismatch` · `StatementPayloadUnavailable { idx }` →
`statement_payload_unavailable` with `{"idx": <u64>}`.

Rust `Debug` output is never used as a code.

## `transport_error`

```json
{"schema":"P10ReplayCliResultV0","status":"transport_error","code":"request_decode_failed","message":"human-readable diagnostic"}
```

`decode_request` failed, so no `ReplayResult` exists. This is not an `invalid` result. `message` is diagnostic only.

## `io_error`

```json
{"schema":"P10ReplayCliResultV0","status":"io_error","code":"output_exists","message":"human-readable diagnostic"}
```

| `code` | exit | meaning |
|---|---|---|
| `output_exists` | 73 | an explicitly named output exists and `--force` was not given |
| `output_cannot_be_created` | 73 | missing/unusable parent directory, non-regular target, or temporary file could not be created |
| `request_read_failed` | 74 | the request file or stdin could not be read |
| `transcript_write_failed` | 73 / 74 | the transcript could not be written; the run is never reported as `verified` |
| `output_write_failed` | 74 | other output failure |

`message` is diagnostic only.
