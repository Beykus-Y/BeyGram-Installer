use plist::{Dictionary, Value};
#[cfg(feature = "async")]
use reqwest::ClientBuilder;
use reqwest::Url;
#[cfg(not(feature = "async"))]
use reqwest::blocking::ClientBuilder;
use std::io::Cursor;

use crate::adi_proxy::ProvisioningError;
use crate::adi_proxy::ProvisioningError::InvalidResponse;

pub(crate) const APPLE_LOOKUP_URL: &str = "https://gsa.apple.com/grandslam/GsService2/lookup";

pub(crate) fn http_builder() -> ClientBuilder {
    // The workspace can enable both AWS-LC and ring through feature unification.
    // Respect an embedding application's provider if it has already installed one.
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    ClientBuilder::new()
        .http1_title_case_headers()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
}

pub(crate) fn parse_dictionary(body: &[u8]) -> Result<Dictionary, ProvisioningError> {
    match Value::from_reader(Cursor::new(body)) {
        Ok(Value::Dictionary(dictionary)) => Ok(dictionary),
        _ => Err(InvalidResponse),
    }
}

pub(crate) fn dictionary_at<'a>(
    dictionary: &'a Dictionary,
    key: &str,
) -> Result<&'a Dictionary, ProvisioningError> {
    dictionary
        .get(key)
        .and_then(Value::as_dictionary)
        .ok_or(InvalidResponse)
}

pub(crate) fn string_at<'a>(
    dictionary: &'a Dictionary,
    key: &str,
) -> Result<&'a str, ProvisioningError> {
    dictionary
        .get(key)
        .and_then(Value::as_string)
        .ok_or(InvalidResponse)
}

pub(crate) fn https_url(value: &str) -> Option<Url> {
    let url = Url::parse(value).ok()?;
    (url.scheme() == "https"
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
        && url.fragment().is_none())
    .then_some(url)
}

pub(crate) fn provisioning_url(lookup: &Dictionary, key: &str) -> Result<Url, ProvisioningError> {
    let value = string_at(dictionary_at(lookup, "urls")?, key)?;
    https_url(value)
        .filter(|url| {
            url.host_str() == Some("gsa.apple.com") && url.port_or_known_default() == Some(443)
        })
        .ok_or(ProvisioningError::UntrustedEndpoint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apple_lookup_is_parsed_and_constrained() {
        let mut lookup =
            parse_dictionary(include_bytes!("../tests/fixtures/apple-lookup.plist")).unwrap();
        assert_eq!(
            provisioning_url(&lookup, "midStartProvisioning")
                .unwrap()
                .host_str(),
            Some("gsa.apple.com")
        );
        for endpoint in [
            "http://gsa.apple.com/grandslam/MidService/startProvisioning",
            "https://gsa.apple.com.example.test/start",
            "https://gsa.apple.com:8443/start",
            "https://user@gsa.apple.com/start",
        ] {
            lookup.insert(
                "urls".into(),
                Value::Dictionary(Dictionary::from_iter([("midStartProvisioning", endpoint)])),
            );
            assert!(matches!(
                provisioning_url(&lookup, "midStartProvisioning"),
                Err(ProvisioningError::UntrustedEndpoint)
            ));
        }
        assert!(parse_dictionary(b"<html>Upstream unavailable</html>").is_err());
        assert!(provisioning_url(&lookup, "midFinishProvisioning").is_err());
    }

    #[test]
    fn missing_local_libraries_do_not_switch_providers() {
        let temp = tempfile::tempdir().unwrap();
        let error = crate::AnisetteHeaders::get_anisette_headers_provider(
            crate::AnisetteConfiguration::new().set_configuration_path(temp.path().join("local")),
        );
        assert!(matches!(error, Err(crate::AnisetteError::MissingLibraries)));
    }
}
