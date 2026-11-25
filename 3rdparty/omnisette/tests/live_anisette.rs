//! Live transport checks. Run explicitly with --ignored.
use omnisette::{AnisetteConfiguration, AnisetteError, AnisetteHeaders};

fn configuration(var: &str) -> AnisetteConfiguration {
    let path = std::env::var_os(var).expect("Set an explicit live provisioning data directory");
    AnisetteConfiguration::new().set_configuration_path(path.into())
}

#[cfg(feature = "async")]
#[tokio::test]
#[ignore = "requires proprietary StoreServicesCore libraries and live Apple provisioning"]
async fn local_store_services_core() -> Result<(), AnisetteError> {
    let mut provider = AnisetteHeaders::get_ssc_anisette_headers_provider(configuration(
        "BEYGRAM_LIVE_LOCAL_DIR",
    ))?;
    let headers = provider.provider.get_authentication_headers().await?;
    assert!(headers.contains_key("X-Apple-I-MD"));
    Ok(())
}

#[cfg(not(feature = "async"))]
#[test]
#[ignore = "requires proprietary StoreServicesCore libraries and live Apple provisioning"]
fn local_store_services_core() -> Result<(), AnisetteError> {
    let mut provider = AnisetteHeaders::get_ssc_anisette_headers_provider(configuration(
        "BEYGRAM_LIVE_LOCAL_DIR",
    ))?;
    let headers = provider.provider.get_authentication_headers()?;
    assert!(headers.contains_key("X-Apple-I-MD"));
    Ok(())
}

#[cfg(feature = "remote-anisette-v3")]
#[tokio::test]
#[ignore = "requires explicit approval of the live Anisette service and Apple provisioning"]
async fn remote_v3() -> Result<(), AnisetteError> {
    let url = std::env::var("BEYGRAM_LIVE_ANISETTE_URL").expect("Set an approved HTTPS endpoint");
    let mut provider = AnisetteHeaders::get_anisette_headers_provider(
        configuration("BEYGRAM_LIVE_REMOTE_DIR").set_remote_anisette_v3(url),
    )?;
    let headers = provider.provider.get_authentication_headers().await?;
    assert!(headers.contains_key("X-Apple-I-MD"));
    Ok(())
}
