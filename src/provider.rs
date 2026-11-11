//! Fail-closed admission checks for a future free-only provider route.
//!
//! This module performs no network requests. A transport may dispatch only
//! after every candidate and fallback in its route has passed this check.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

const REQUIRED_PRICE_CATEGORIES: [&str; 7] = [
    "prompt",
    "completion",
    "request",
    "image",
    "audio_input",
    "audio_output",
    "cache",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreeCandidate {
    pub model_id: String,
    pub provider_id: String,
    pub free_variant: bool,
    /// Every provider-documented charge category. Missing values are unknown.
    pub prices_usd: BTreeMap<String, Option<String>>,
    pub data_collection: Option<String>,
    pub retention_days: Option<u32>,
    pub terms_compatible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionRequest {
    /// Exact model IDs admitted from a timestamped catalogue snapshot.
    pub allowlist: BTreeSet<String>,
    /// Exact providers admitted for every model and fallback.
    pub provider_allowlist: BTreeSet<String>,
    /// Primary candidate followed by every possible fallback.
    pub candidates: Vec<FreeCandidate>,
    pub catalogue_expires_at_unix: u64,
    pub now_unix: u64,
    pub maximum_retention_days: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedRoute {
    pub targets: Vec<AdmittedTarget>,
    pub monetary_ceiling_usd: String,
    pub required_data_collection: String,
    pub maximum_retention_days: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedTarget {
    pub model_id: String,
    pub provider_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionError {
    EmptyRoute,
    StaleCatalogue,
    EmptyAllowlist,
    EmptyProviderAllowlist,
    DuplicateModel(String),
    ModelNotAllowlisted(String),
    ProviderNotAllowlisted(String),
    NotExplicitlyFree(String),
    MissingPrice(String, String),
    NonZeroPrice(String, String),
    PrivacyNotDenied(String),
    RetentionUnknown(String),
    RetentionTooLong(String),
    TermsIncompatible(String),
}

impl fmt::Display for AdmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for AdmissionError {}

pub fn admit_free_route(request: &AdmissionRequest) -> Result<AdmittedRoute, AdmissionError> {
    if request.candidates.is_empty() {
        return Err(AdmissionError::EmptyRoute);
    }
    if request.now_unix >= request.catalogue_expires_at_unix {
        return Err(AdmissionError::StaleCatalogue);
    }
    if request.allowlist.is_empty() {
        return Err(AdmissionError::EmptyAllowlist);
    }
    if request.provider_allowlist.is_empty() {
        return Err(AdmissionError::EmptyProviderAllowlist);
    }

    let mut seen = BTreeSet::new();
    let mut targets = Vec::with_capacity(request.candidates.len());
    for candidate in &request.candidates {
        if !seen.insert(candidate.model_id.clone()) {
            return Err(AdmissionError::DuplicateModel(candidate.model_id.clone()));
        }
        if !request.allowlist.contains(&candidate.model_id) {
            return Err(AdmissionError::ModelNotAllowlisted(
                candidate.model_id.clone(),
            ));
        }
        if candidate.provider_id.trim().is_empty()
            || !request.provider_allowlist.contains(&candidate.provider_id)
        {
            return Err(AdmissionError::ProviderNotAllowlisted(
                candidate.provider_id.clone(),
            ));
        }
        if !candidate.free_variant || candidate.model_id == "openrouter/free" {
            return Err(AdmissionError::NotExplicitlyFree(
                candidate.model_id.clone(),
            ));
        }
        if !candidate.terms_compatible {
            return Err(AdmissionError::TermsIncompatible(
                candidate.model_id.clone(),
            ));
        }
        if candidate.data_collection.as_deref() != Some("deny") {
            return Err(AdmissionError::PrivacyNotDenied(candidate.model_id.clone()));
        }
        match candidate.retention_days {
            None => return Err(AdmissionError::RetentionUnknown(candidate.model_id.clone())),
            Some(days) if days > request.maximum_retention_days => {
                return Err(AdmissionError::RetentionTooLong(candidate.model_id.clone()));
            }
            Some(_) => {}
        }
        for category in REQUIRED_PRICE_CATEGORIES {
            validate_price(candidate, category)?;
        }
        for (category, price) in &candidate.prices_usd {
            match price {
                None => {
                    return Err(AdmissionError::MissingPrice(
                        candidate.model_id.clone(),
                        category.clone(),
                    ));
                }
                Some(value) if !is_explicit_zero(value) => {
                    return Err(AdmissionError::NonZeroPrice(
                        candidate.model_id.clone(),
                        category.clone(),
                    ));
                }
                Some(_) => {}
            }
        }
        targets.push(AdmittedTarget {
            model_id: candidate.model_id.clone(),
            provider_id: candidate.provider_id.clone(),
        });
    }

    Ok(AdmittedRoute {
        targets,
        monetary_ceiling_usd: "0".into(),
        required_data_collection: "deny".into(),
        maximum_retention_days: request.maximum_retention_days,
    })
}

fn validate_price(candidate: &FreeCandidate, category: &str) -> Result<(), AdmissionError> {
    match candidate.prices_usd.get(category) {
        None | Some(None) => Err(AdmissionError::MissingPrice(
            candidate.model_id.clone(),
            category.into(),
        )),
        Some(Some(value)) if !is_explicit_zero(value) => Err(AdmissionError::NonZeroPrice(
            candidate.model_id.clone(),
            category.into(),
        )),
        Some(Some(_)) => Ok(()),
    }
}

fn is_explicit_zero(value: &str) -> bool {
    let mut decimal = false;
    let mut digits = 0;
    for byte in value.bytes() {
        match byte {
            b'0' => digits += 1,
            b'.' if !decimal => decimal = true,
            _ => return false,
        }
    }
    digits > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn free_candidate(model_id: &str) -> FreeCandidate {
        let prices_usd = REQUIRED_PRICE_CATEGORIES
            .into_iter()
            .map(|category| (category.into(), Some("0".into())))
            .collect();
        FreeCandidate {
            model_id: model_id.into(),
            provider_id: "provider-a".into(),
            free_variant: true,
            prices_usd,
            data_collection: Some("deny".into()),
            retention_days: Some(0),
            terms_compatible: true,
        }
    }

    fn request(candidates: Vec<FreeCandidate>) -> AdmissionRequest {
        AdmissionRequest {
            allowlist: candidates
                .iter()
                .map(|candidate| candidate.model_id.clone())
                .collect(),
            provider_allowlist: candidates
                .iter()
                .map(|candidate| candidate.provider_id.clone())
                .collect(),
            candidates,
            catalogue_expires_at_unix: 200,
            now_unix: 100,
            maximum_retention_days: 7,
        }
    }

    #[test]
    fn admits_only_exact_free_candidates_with_zero_cost_and_privacy_limits() {
        let route = admit_free_route(&request(vec![free_candidate("model-a:free")])).unwrap();
        assert_eq!(route.targets[0].model_id, "model-a:free");
        assert_eq!(route.targets[0].provider_id, "provider-a");
        assert_eq!(route.monetary_ceiling_usd, "0");
    }

    #[test]
    fn rejects_unknown_price_even_when_other_categories_are_zero() {
        let mut candidate = free_candidate("model-a:free");
        candidate.prices_usd.insert("audio_output".into(), None);
        assert_eq!(
            admit_free_route(&request(vec![candidate])).unwrap_err(),
            AdmissionError::MissingPrice("model-a:free".into(), "audio_output".into())
        );
    }

    #[test]
    fn rejects_a_paid_or_dynamic_fallback() {
        let mut fallback = free_candidate("model-b");
        fallback.free_variant = false;
        assert!(matches!(
            admit_free_route(&request(vec![free_candidate("model-a:free"), fallback])),
            Err(AdmissionError::NotExplicitlyFree(_))
        ));
    }

    #[test]
    fn rejects_stale_catalogues_and_unrestricted_privacy() {
        let mut req = request(vec![free_candidate("model-a:free")]);
        req.catalogue_expires_at_unix = 100;
        assert_eq!(admit_free_route(&req), Err(AdmissionError::StaleCatalogue));

        let mut candidate = free_candidate("model-a:free");
        candidate.data_collection = Some("allow".into());
        assert!(matches!(
            admit_free_route(&request(vec![candidate])),
            Err(AdmissionError::PrivacyNotDenied(_))
        ));
    }

    #[test]
    fn rejects_an_empty_provider_allowlist() {
        let mut req = request(vec![free_candidate("model-a:free")]);
        req.provider_allowlist.clear();
        assert_eq!(
            admit_free_route(&req),
            Err(AdmissionError::EmptyProviderAllowlist)
        );
    }

    #[test]
    fn rejects_dynamic_routing_and_unapproved_providers() {
        let candidate = free_candidate("openrouter/free");
        assert!(matches!(
            admit_free_route(&request(vec![candidate])),
            Err(AdmissionError::NotExplicitlyFree(_))
        ));

        let mut req = request(vec![free_candidate("model-a:free")]);
        req.provider_allowlist = ["provider-b".into()].into();
        assert!(matches!(
            admit_free_route(&req),
            Err(AdmissionError::ProviderNotAllowlisted(_))
        ));
    }
}
