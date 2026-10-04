//! CLI-owned result JSON (`P10ReplayCliResultV0`). Pure mapping of already-returned core values; nothing
//! here chooses an outcome.

use p10_replay::evidence::ReplayEvidence;
use p10_replay::replay::{Failure, Phase, Reason, ResultClass, Severity};

use crate::json::Json;

pub const SCHEMA: &str = "P10ReplayCliResultV0";

// Every mapping below is an explicit exhaustive `match` with no wildcard arm, so a new core variant is a
// compile error here rather than a silently unmapped code.

pub fn phase_code(p: Phase) -> &'static str {
    match p {
        Phase::P0 => "p0",
        Phase::P1 => "p1",
        Phase::P2 => "p2",
        Phase::P3 => "p3",
        Phase::P4 => "p4",
        Phase::P5 => "p5",
        Phase::P6 => "p6",
        Phase::P7 => "p7",
        Phase::P8 => "p8",
        Phase::P9 => "p9",
    }
}

pub fn severity_code(s: Severity) -> &'static str {
    match s {
        Severity::Invalid => "invalid",
        Severity::Unavailable => "unavailable",
    }
}

pub fn class_code(c: ResultClass) -> &'static str {
    match c {
        ResultClass::Request => "request",
        ResultClass::Replay => "replay",
        ResultClass::Binding => "binding",
        ResultClass::Material => "material",
        ResultClass::Capability => "capability",
    }
}

pub fn reason_code(r: &Reason) -> &'static str {
    match r {
        Reason::FrozenConfigMalformed => "frozen_config_malformed",
        Reason::KeyMaterialFingerprintMismatch => "key_material_fingerprint_mismatch",
        Reason::VerifierCapabilityUnsupported => "verifier_capability_unsupported",
        Reason::ReceiptAbsent => "receipt_absent",
        Reason::TargetStatementAbsent => "target_statement_absent",
        Reason::ReceiptMalformed => "receipt_malformed",
        Reason::ReceiptTargetBindingFailure => "receipt_target_binding_failure",
        Reason::ReceiptTargetAmbiguous => "receipt_target_ambiguous",
        Reason::ReceiptTargetAuthenticationFailure => "receipt_target_authentication_failure",
        Reason::ReceiptBadSignature => "receipt_bad_signature",
        Reason::ReceiptVdsUnsupported => "receipt_vds_unsupported",
        Reason::ReceiptKeyUnavailable => "receipt_key_unavailable",
        Reason::VdsAlgorithmMismatch => "vds_algorithm_mismatch",
        Reason::DuplicateIndex => "duplicate_index",
        Reason::IndexOutOfPrefix => "index_out_of_prefix",
        Reason::MissingIndex { .. } => "missing_index",
        Reason::RequestMalformed => "request_malformed",
        Reason::TargetStatementMismatch => "target_statement_mismatch",
        Reason::RootMismatch => "root_mismatch",
        Reason::TsIssuerMismatch => "ts_issuer_mismatch",
        Reason::LogIdentityInconsistent => "log_identity_inconsistent",
        Reason::LeafSpecDigestMismatch => "leaf_spec_digest_mismatch",
        Reason::LeafInputDomainViolation => "leaf_input_domain_violation",
        Reason::TargetKindMismatch => "target_kind_mismatch",
        Reason::StatementPayloadUnavailable { .. } => "statement_payload_unavailable",
    }
}

fn reason_parameters(r: &Reason) -> Json {
    match r {
        Reason::MissingIndex { first_missing } => {
            Json::Obj(vec![("first_missing", Json::Num(*first_missing))])
        }
        Reason::StatementPayloadUnavailable { idx } => Json::Obj(vec![("idx", Json::Num(*idx))]),
        Reason::FrozenConfigMalformed
        | Reason::KeyMaterialFingerprintMismatch
        | Reason::VerifierCapabilityUnsupported
        | Reason::ReceiptAbsent
        | Reason::TargetStatementAbsent
        | Reason::ReceiptMalformed
        | Reason::ReceiptTargetBindingFailure
        | Reason::ReceiptTargetAmbiguous
        | Reason::ReceiptTargetAuthenticationFailure
        | Reason::ReceiptBadSignature
        | Reason::ReceiptVdsUnsupported
        | Reason::ReceiptKeyUnavailable
        | Reason::VdsAlgorithmMismatch
        | Reason::DuplicateIndex
        | Reason::IndexOutOfPrefix
        | Reason::RequestMalformed
        | Reason::TargetStatementMismatch
        | Reason::RootMismatch
        | Reason::TsIssuerMismatch
        | Reason::LogIdentityInconsistent
        | Reason::LeafSpecDigestMismatch
        | Reason::LeafInputDomainViolation
        | Reason::TargetKindMismatch => Json::Obj(vec![]),
    }
}

fn failure(f: &Failure) -> Json {
    Json::Obj(vec![
        ("phase", Json::str(phase_code(f.phase))),
        ("severity", Json::str(severity_code(f.severity))),
        ("class", Json::str(class_code(f.class))),
        ("reason", Json::str(reason_code(&f.reason))),
        ("parameters", reason_parameters(&f.reason)),
        (
            "detail",
            f.detail
                .as_ref()
                .map_or(Json::Null, |d| Json::str(d.clone())),
        ),
    ])
}

pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(DIGITS[(b >> 4) as usize] as char);
        s.push(DIGITS[(b & 0xf) as usize] as char);
    }
    s
}

fn head(status: &str) -> Vec<(&'static str, Json)> {
    vec![("schema", Json::str(SCHEMA)), ("status", Json::str(status))]
}

/// `transcript_len` is the length of the bytes handed to the writer; the hash is the core's own.
pub fn verified(transcript_len: usize, ev: &ReplayEvidence) -> Json {
    let mut o = head("verified");
    o.push((
        "transcript",
        Json::Obj(vec![
            ("bytes", Json::Num(transcript_len as u64)),
            ("sha256", Json::str(hex(&ev.transcript_sha256))),
        ]),
    ));
    o.push((
        "evidence_summary",
        Json::Obj(vec![
            ("checkpoint_size", Json::Num(ev.checkpoint_size)),
            ("root_digest", Json::str(hex(&ev.root_digest))),
            (
                "log_identity_digest",
                Json::str(hex(&ev.log_identity.log_identity_digest)),
            ),
            (
                "statement_count",
                Json::Num(ev.statement_sha256.len() as u64),
            ),
            (
                "subject_observation_count",
                Json::Num(ev.subject_observations.len() as u64),
            ),
            (
                "unreadable_row_count",
                Json::Num(ev.unreadable_rows.len() as u64),
            ),
        ]),
    ));
    Json::Obj(o)
}

/// `status` is "invalid" or "unavailable", chosen by the caller from the `ReplayResult` variant.
pub fn failed(status: &str, primary: &Failure, diagnostics: &[Failure]) -> Json {
    let mut o = head(status);
    o.push(("primary", failure(primary)));
    o.push((
        "diagnostics",
        Json::Arr(diagnostics.iter().map(failure).collect()),
    ));
    Json::Obj(o)
}

pub fn transport_error(code: &str, message: &str) -> Json {
    let mut o = head("transport_error");
    o.push(("code", Json::str(code)));
    o.push(("message", Json::str(message)));
    Json::Obj(o)
}

pub fn io_error(code: &str, message: &str) -> Json {
    let mut o = head("io_error");
    o.push(("code", Json::str(code)));
    o.push(("message", Json::str(message)));
    Json::Obj(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every public core `Reason` variant, with its expected stable code. The exhaustive matches above make
    /// adding a variant a compile error; this table pins the exact strings and checks uniqueness.
    fn all_reasons() -> Vec<(Reason, &'static str)> {
        vec![
            (Reason::FrozenConfigMalformed, "frozen_config_malformed"),
            (
                Reason::KeyMaterialFingerprintMismatch,
                "key_material_fingerprint_mismatch",
            ),
            (
                Reason::VerifierCapabilityUnsupported,
                "verifier_capability_unsupported",
            ),
            (Reason::ReceiptAbsent, "receipt_absent"),
            (Reason::TargetStatementAbsent, "target_statement_absent"),
            (Reason::ReceiptMalformed, "receipt_malformed"),
            (
                Reason::ReceiptTargetBindingFailure,
                "receipt_target_binding_failure",
            ),
            (Reason::ReceiptTargetAmbiguous, "receipt_target_ambiguous"),
            (
                Reason::ReceiptTargetAuthenticationFailure,
                "receipt_target_authentication_failure",
            ),
            (Reason::ReceiptBadSignature, "receipt_bad_signature"),
            (Reason::ReceiptVdsUnsupported, "receipt_vds_unsupported"),
            (Reason::ReceiptKeyUnavailable, "receipt_key_unavailable"),
            (Reason::VdsAlgorithmMismatch, "vds_algorithm_mismatch"),
            (Reason::DuplicateIndex, "duplicate_index"),
            (Reason::IndexOutOfPrefix, "index_out_of_prefix"),
            (Reason::MissingIndex { first_missing: 4 }, "missing_index"),
            (Reason::RequestMalformed, "request_malformed"),
            (Reason::TargetStatementMismatch, "target_statement_mismatch"),
            (Reason::RootMismatch, "root_mismatch"),
            (Reason::TsIssuerMismatch, "ts_issuer_mismatch"),
            (Reason::LogIdentityInconsistent, "log_identity_inconsistent"),
            (Reason::LeafSpecDigestMismatch, "leaf_spec_digest_mismatch"),
            (
                Reason::LeafInputDomainViolation,
                "leaf_input_domain_violation",
            ),
            (Reason::TargetKindMismatch, "target_kind_mismatch"),
            (
                Reason::StatementPayloadUnavailable { idx: 7 },
                "statement_payload_unavailable",
            ),
        ]
    }

    #[test]
    fn every_reason_has_its_stable_code() {
        let table = all_reasons();
        assert_eq!(table.len(), 25);
        let mut seen = std::collections::BTreeSet::new();
        for (r, code) in &table {
            assert_eq!(reason_code(r), *code);
            assert!(
                code.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'),
                "{code}"
            );
            assert!(seen.insert(*code), "duplicate code {code}");
        }
    }

    #[test]
    fn parameterized_reasons() {
        assert_eq!(
            reason_parameters(&Reason::MissingIndex { first_missing: 4 }).to_line(),
            "{\"first_missing\":4}\n"
        );
        assert_eq!(
            reason_parameters(&Reason::StatementPayloadUnavailable { idx: 7 }).to_line(),
            "{\"idx\":7}\n"
        );
        assert_eq!(reason_parameters(&Reason::RootMismatch).to_line(), "{}\n");
    }

    #[test]
    fn phase_severity_class_codes() {
        let phases = [
            Phase::P0,
            Phase::P1,
            Phase::P2,
            Phase::P3,
            Phase::P4,
            Phase::P5,
            Phase::P6,
            Phase::P7,
            Phase::P8,
            Phase::P9,
        ];
        for (i, p) in phases.iter().enumerate() {
            assert_eq!(phase_code(*p), format!("p{i}"));
        }
        assert_eq!(severity_code(Severity::Invalid), "invalid");
        assert_eq!(severity_code(Severity::Unavailable), "unavailable");
        let classes = [
            (ResultClass::Request, "request"),
            (ResultClass::Replay, "replay"),
            (ResultClass::Binding, "binding"),
            (ResultClass::Material, "material"),
            (ResultClass::Capability, "capability"),
        ];
        for (c, s) in classes {
            assert_eq!(class_code(c), s);
        }
    }

    #[test]
    fn hex_is_lowercase_fixed_width() {
        assert_eq!(hex(&[0x00, 0x0f, 0xa0, 0xff]), "000fa0ff");
    }
}
