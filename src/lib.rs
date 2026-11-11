//! Local, deterministic emo baseline.
//!
//! Version 0.0.1 deliberately offers a no-op decision path. No message is
//! rewritten until a bounded transformation has independently passed the
//! preservation gates described in the project directive.

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::fmt;

pub const DECISION_SCHEMA: &str = "emo.decision.v1";
pub const REQUEST_SCHEMA: &str = "emo.request.v1";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub request_id: String,
    pub session_id: String,
    pub message: String,
    #[serde(default)]
    pub language: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Decision {
    pub schema: String,
    pub request_id: String,
    pub session_id: String,
    pub status: DecisionStatus,
    pub reason_codes: Vec<String>,
    pub output: String,
    pub original_digest: String,
    pub output_digest: String,
    pub policy_digest: Option<String>,
    pub edits: Vec<Edit>,
    pub invariants: InvariantResults,
    pub usage: Usage,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStatus {
    Noop,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Edit {
    pub start_byte: usize,
    pub end_byte: usize,
    pub replacement: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct InvariantResults {
    pub original_preserved: bool,
    pub protected_bytes_preserved: bool,
    pub permissions_unchanged: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Usage {
    pub provider_cost_usd: Option<f64>,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub schema: String,
    pub policy_id: String,
    pub version: String,
    pub state: String,
    pub permitted_edits: Vec<String>,
    pub scope: PolicyScope,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PolicyScope {
    pub languages: Vec<String>,
    pub task_types: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidInput(String),
    InvalidPolicy(String),
    EntropyUnavailable,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(message) | Self::InvalidPolicy(message) => f.write_str(message),
            Self::EntropyUnavailable => {
                f.write_str("secure random source unavailable; no decision was emitted")
            }
        }
    }
}

impl std::error::Error for Error {}

pub fn decide(request: Request) -> Result<Decision, Error> {
    if request.schema != REQUEST_SCHEMA {
        return Err(Error::InvalidInput(format!(
            "schema must be {REQUEST_SCHEMA}"
        )));
    }
    if request.request_id.trim().is_empty() || request.session_id.trim().is_empty() {
        return Err(Error::InvalidInput(
            "request_id and session_id must be non-empty".into(),
        ));
    }
    if request.message.len() > 16 * 1024 {
        return Err(Error::InvalidInput(
            "message exceeds the 16 KiB limit".into(),
        ));
    }
    let digest_key = ephemeral_key()?;
    let digest = digest(&request.message, &digest_key);
    Ok(Decision {
        schema: DECISION_SCHEMA.into(),
        request_id: request.request_id,
        session_id: request.session_id,
        status: DecisionStatus::Noop,
        reason_codes: vec!["NO_QUALIFIED_POLICY".into()],
        output: request.message,
        original_digest: digest.clone(),
        output_digest: digest,
        policy_digest: None,
        edits: Vec::new(),
        invariants: InvariantResults {
            original_preserved: true,
            protected_bytes_preserved: true,
            permissions_unchanged: true,
        },
        usage: Usage {
            provider_cost_usd: None,
            status: "not_invoked".into(),
        },
        evidence_refs: Vec::new(),
    })
}

pub fn validate_policy(policy: &Policy) -> Result<(), Error> {
    if policy.schema != "emo.policy.v1" {
        return Err(Error::InvalidPolicy("schema must be emo.policy.v1".into()));
    }
    if policy.policy_id.trim().is_empty() || policy.version.trim().is_empty() {
        return Err(Error::InvalidPolicy(
            "policy_id and version must be non-empty".into(),
        ));
    }
    if !matches!(
        policy.state.as_str(),
        "candidate" | "reviewed" | "qualified" | "active" | "revoked" | "expired"
    ) {
        return Err(Error::InvalidPolicy("unknown policy state".into()));
    }
    if policy.permitted_edits.iter().any(|edit| edit != "noop") {
        return Err(Error::InvalidPolicy(
            "v0.0.1 accepts only the noop edit".into(),
        ));
    }
    if policy.scope.languages.is_empty() || policy.scope.task_types.is_empty() {
        return Err(Error::InvalidPolicy(
            "policy scope must name languages and task_types".into(),
        ));
    }
    Ok(())
}

fn ephemeral_key() -> Result<[u8; 32], Error> {
    let mut key = [0_u8; 32];
    getrandom::getrandom(&mut key).map_err(|_| Error::EntropyUnavailable)?;
    Ok(key)
}

fn digest(value: &str, key: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts a 32-byte key");
    mac.update(value.as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(message: &str) -> Request {
        Request {
            schema: REQUEST_SCHEMA.into(),
            request_id: "req-test".into(),
            session_id: "session-test".into(),
            message: message.into(),
            language: None,
        }
    }

    #[test]
    fn no_op_preserves_unicode_whitespace_and_instruction_bytes() {
        let source = "Do not deploy.\n\t8 GiB — continue 🙂";
        let decision = decide(request(source)).unwrap();
        assert_eq!(decision.output.as_bytes(), source.as_bytes());
        assert_eq!(decision.original_digest, decision.output_digest);
        assert!(decision.edits.is_empty());
        assert_eq!(decision.status, DecisionStatus::Noop);
    }

    #[test]
    fn oversized_and_incompatible_requests_are_rejected() {
        let mut bad_schema = request("hello");
        bad_schema.schema = "emo.request.v2".into();
        assert!(decide(bad_schema).is_err());
        assert!(decide(request(&"x".repeat(16 * 1024 + 1))).is_err());
    }

    #[test]
    fn request_deserialization_rejects_unknown_fields() {
        let input = r#"{"schema":"emo.request.v1","request_id":"r","session_id":"s","message":"x","sudo":true}"#;
        assert!(serde_json::from_str::<Request>(input).is_err());
    }

    #[test]
    fn non_noop_policy_is_never_accepted_as_qualified() {
        let policy = Policy {
            schema: "emo.policy.v1".into(),
            policy_id: "courtesy".into(),
            version: "1.0.0".into(),
            state: "qualified".into(),
            permitted_edits: vec!["replace_mutable_span".into()],
            scope: PolicyScope {
                languages: vec!["en".into()],
                task_types: vec!["code".into()],
            },
        };
        assert!(validate_policy(&policy).is_err());
    }
}
