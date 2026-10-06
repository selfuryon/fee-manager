// validation.rs - Write-time checks for values that end up in Vouch's execution config
//
// Types stay plain strings so that rows stored before these checks existed
// still decode and keep being served; validation runs in the handlers instead.
use crate::errors::ApiError;
use crate::schema::{
    CreateDefaultConfigRequest, CreateOrUpdateProposerRequest, CreateProposerPatternRequest,
    ProposerRelayConfig, RelayConfig, UpdateDefaultConfigRequest, UpdateProposerPatternRequest,
};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::str::FromStr;

/// Positive integer in plain decimal digits that fits in u64.
fn gas_limit(value: &str) -> Result<(), String> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err("must be a positive integer in decimal digits".into());
    }
    match value.parse::<u64>() {
        Ok(0) => Err("must be greater than 0".into()),
        Ok(_) => Ok(()),
        Err(_) => Err("is too large".into()),
    }
}

/// Non-negative decimal amount of ETH: digits with an optional fractional part.
fn min_value(value: &str) -> Result<(), String> {
    let (int, frac) = value.split_once('.').unwrap_or((value, "1"));
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(int) || !digits(frac) {
        return Err("must be a non-negative decimal number, e.g. \"0.1\"".into());
    }
    Decimal::from_str(value)
        .map(|_| ())
        .map_err(|_| "is out of range".into())
}

/// Absolute http(s) URL with a host.
fn relay_url(value: &str) -> Result<(), String> {
    let url = url::Url::parse(value).map_err(|e| format!("is not a valid URL: {e}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("must use http or https".into());
    }
    if url.host_str().is_none_or(str::is_empty) {
        return Err("must include a host".into());
    }
    Ok(())
}

/// A regular expression that compiles. Rust's regex follows RE2 syntax, the
/// family Go's regexp (used by Vouch) belongs to.
fn pattern(value: &str) -> Result<(), String> {
    regex::Regex::new(value)
        .map(|_| ())
        .map_err(|e| format!("is not a valid regular expression: {e}"))
}

fn check(
    field: &str,
    value: Option<&str>,
    rule: fn(&str) -> Result<(), String>,
) -> Result<(), ApiError> {
    match value {
        Some(v) => rule(v).map_err(|reason| ApiError::InvalidData(format!("{field} {reason}"))),
        None => Ok(()),
    }
}

/// Fields shared by both relay types.
trait RelayFields {
    fn gas_limit(&self) -> Option<&str>;
    fn min_value(&self) -> Option<&str>;
}

impl RelayFields for RelayConfig {
    fn gas_limit(&self) -> Option<&str> {
        self.gas_limit.as_deref()
    }
    fn min_value(&self) -> Option<&str> {
        self.min_value.as_deref()
    }
}

impl RelayFields for ProposerRelayConfig {
    fn gas_limit(&self) -> Option<&str> {
        self.gas_limit.as_deref()
    }
    fn min_value(&self) -> Option<&str> {
        self.min_value.as_deref()
    }
}

fn relays<R: RelayFields>(relays: Option<&HashMap<String, R>>) -> Result<(), ApiError> {
    let Some(relays) = relays else { return Ok(()) };
    // Sorted so the reported error does not depend on hash order
    let mut urls: Vec<&String> = relays.keys().collect();
    urls.sort();
    for url in urls {
        let relay = &relays[url];
        let prefix = |e: ApiError| match e {
            ApiError::InvalidData(msg) => ApiError::InvalidData(format!("relay {url}: {msg}")),
            other => other,
        };
        check("url", Some(url), relay_url).map_err(prefix)?;
        check("gas_limit", relay.gas_limit(), gas_limit).map_err(prefix)?;
        check("min_value", relay.min_value(), min_value).map_err(prefix)?;
    }
    Ok(())
}

impl CreateDefaultConfigRequest {
    pub fn validate(&self) -> Result<(), ApiError> {
        check("gas_limit", self.gas_limit.as_deref(), gas_limit)?;
        check("min_value", self.min_value.as_deref(), min_value)?;
        relays(self.relays.as_ref())
    }
}

impl UpdateDefaultConfigRequest {
    pub fn validate(&self) -> Result<(), ApiError> {
        check("gas_limit", self.gas_limit.as_deref(), gas_limit)?;
        check("min_value", self.min_value.as_deref(), min_value)?;
        relays(self.relays.as_ref())
    }
}

impl CreateOrUpdateProposerRequest {
    pub fn validate(&self) -> Result<(), ApiError> {
        check("gas_limit", self.gas_limit.as_deref(), gas_limit)?;
        check("min_value", self.min_value.as_deref(), min_value)?;
        relays(self.relays.as_ref())
    }
}

impl CreateProposerPatternRequest {
    pub fn validate(&self) -> Result<(), ApiError> {
        check("pattern", Some(&self.pattern), pattern)?;
        check("gas_limit", self.gas_limit.as_deref(), gas_limit)?;
        check("min_value", self.min_value.as_deref(), min_value)?;
        relays(self.relays.as_ref())
    }
}

impl UpdateProposerPatternRequest {
    pub fn validate(&self) -> Result<(), ApiError> {
        check("pattern", self.pattern.as_deref(), pattern)?;
        check("gas_limit", self.gas_limit.as_deref(), gas_limit)?;
        check("min_value", self.min_value.as_deref(), min_value)?;
        relays(self.relays.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gas_limit_rules() {
        assert!(gas_limit("30000000").is_ok());
        assert!(gas_limit("18446744073709551615").is_ok());
        for bad in [
            "0",
            "lots",
            " 1",
            "+1",
            "1e3",
            "1.0",
            "",
            "18446744073709551616",
        ] {
            assert!(gas_limit(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn min_value_rules() {
        for good in ["0", "0.10", "1", "12.5"] {
            assert!(min_value(good).is_ok(), "{good:?} should be accepted");
        }
        for bad in ["-1", "cheap", "1e-3", "", ".5", "1.", "1.2.3", " 1", "+1"] {
            assert!(min_value(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn relay_url_rules() {
        assert!(relay_url("https://relay.example.invalid/").is_ok());
        assert!(relay_url("http://relay.example.invalid:8080").is_ok());
        for bad in [
            "relay.example.invalid",
            "ftp://relay.example.invalid/",
            "https://",
            "",
        ] {
            assert!(relay_url(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn pattern_rules() {
        assert!(pattern("^pool/.*$").is_ok());
        assert!(pattern("([unclosed").is_err());
    }

    fn relay(gas: Option<&str>, min: Option<&str>) -> RelayConfig {
        RelayConfig {
            public_key: serde_json::from_str(&format!("\"0x{}\"", "ab".repeat(48))).unwrap(),
            fee_recipient: None,
            gas_limit: gas.map(Into::into),
            min_value: min.map(Into::into),
            disabled: false,
        }
    }

    fn message(err: ApiError) -> String {
        match err {
            ApiError::InvalidData(msg) => msg,
            other => panic!("unexpected error {other:?}"),
        }
    }

    #[test]
    fn relay_error_names_field_and_url() {
        let req = UpdateDefaultConfigRequest {
            fee_recipient: None,
            gas_limit: None,
            min_value: None,
            active: None,
            relays: Some(HashMap::from([(
                "https://relay.example.invalid/".to_string(),
                relay(None, Some("cheap")),
            )])),
        };
        let msg = message(req.validate().unwrap_err());
        assert!(
            msg.contains("min_value") && msg.contains("https://relay.example.invalid/"),
            "{msg}"
        );
    }

    #[test]
    fn omitted_fields_pass() {
        let req = UpdateDefaultConfigRequest {
            fee_recipient: None,
            gas_limit: None,
            min_value: None,
            active: Some(false),
            relays: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn top_level_error_names_field() {
        let req = UpdateDefaultConfigRequest {
            fee_recipient: None,
            gas_limit: Some("lots".into()),
            min_value: None,
            active: None,
            relays: Some(HashMap::from([(
                "https://relay.example.invalid/".to_string(),
                relay(Some("30000000"), None),
            )])),
        };
        assert!(message(req.validate().unwrap_err()).starts_with("gas_limit "));
    }
}
