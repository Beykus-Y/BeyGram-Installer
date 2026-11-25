//! A library to generate "anisette" data. Docs are coming soon.
//!
//! If you want an async API, enable the `async` feature.
//!
//! If you want remote anisette, make sure the `remote-anisette` feature is enabled. (it's currently on by default)

use crate::adi_proxy::{ADIProxyAnisetteProvider, ConfigurableADIProxy};
use crate::anisette_headers_provider::AnisetteHeadersProvider;
use adi_proxy::ADIError;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

pub mod adi_proxy;
pub mod anisette_headers_provider;
pub mod store_services_core;
mod transport;

#[cfg(feature = "remote-anisette-v3")]
pub mod remote_anisette_v3;

#[allow(dead_code)]
pub struct AnisetteHeaders;

#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum AnisetteError {
    #[allow(dead_code)]
    #[error("Unsupported device")]
    UnsupportedDevice,
    #[error("Invalid argument {0}")]
    InvalidArgument(String),
    #[error("Anisette not provisioned!")]
    AnisetteNotProvisioned,
    #[error("Plist serialization error {0}")]
    PlistError(#[from] plist::Error),
    #[error("Request Error {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[cfg(feature = "remote-anisette-v3")]
    #[error("Provisioning socket error {0}")]
    WsError(#[from] tokio_tungstenite::tungstenite::error::Error),
    #[cfg(feature = "remote-anisette-v3")]
    #[error("JSON error {0}")]
    SerdeError(#[from] serde_json::Error),
    #[error("Anisette provisioning failed: {0}")]
    Provisioning(#[from] adi_proxy::ProvisioningError),
    #[error(transparent)]
    SecretStorage(#[from] plume_secrets::Error),
    #[error("IO error {0}")]
    IOError(#[from] io::Error),
    #[error("ADI error {0}")]
    ADIError(#[from] ADIError),
    #[error("Invalid library format")]
    InvalidLibraryFormat,
    #[error("Misc")]
    Misc,
    #[error("Local Anisette requires StoreServicesCore libraries. See docs/BUILD.md for setup.")]
    MissingLibraries,
    #[error("{0}")]
    Anyhow(#[from] anyhow::Error),
}

pub const DEFAULT_ANISETTE_URL: &str = "https://ani.f1sh.me/";

pub const DEFAULT_ANISETTE_URL_V3: &str = "https://mac-ani.jkcoxson.com";

#[derive(Clone, Debug)]
pub struct AnisetteConfiguration {
    anisette_url: String,
    #[cfg(feature = "remote-anisette-v3")]
    anisette_url_v3: Option<String>,
    configuration_path: PathBuf,
    macos_serial: String,
}

impl Default for AnisetteConfiguration {
    fn default() -> Self {
        AnisetteConfiguration::new()
    }
}

impl AnisetteConfiguration {
    pub fn new() -> AnisetteConfiguration {
        AnisetteConfiguration {
            anisette_url: DEFAULT_ANISETTE_URL.to_string(),
            #[cfg(feature = "remote-anisette-v3")]
            anisette_url_v3: None,
            configuration_path: PathBuf::new(),
            macos_serial: "0".to_string(),
        }
    }

    pub fn anisette_url(&self) -> &String {
        &self.anisette_url
    }

    pub fn configuration_path(&self) -> &PathBuf {
        &self.configuration_path
    }

    pub fn set_anisette_url(mut self, anisette_url: String) -> AnisetteConfiguration {
        self.anisette_url = anisette_url;
        self
    }

    /// Select an approved remote HTTPS provider instead of local StoreServicesCore.
    ///
    /// The service receives the provisioning identifier, Apple provisioning
    /// intermediates, and the ADI provisioning blob. It also sees the client's IP.
    #[cfg(feature = "remote-anisette-v3")]
    pub fn set_remote_anisette_v3(mut self, url: String) -> Self {
        self.anisette_url_v3 = Some(url);
        self
    }

    pub fn set_macos_serial(mut self, macos_serial: String) -> AnisetteConfiguration {
        self.macos_serial = macos_serial;
        self
    }

    pub fn set_configuration_path(mut self, configuration_path: PathBuf) -> AnisetteConfiguration {
        self.configuration_path = configuration_path;
        self
    }
}

pub enum AnisetteHeadersProviderType {
    Local,
    Remote,
}

pub struct AnisetteHeadersProviderRes {
    pub provider: Box<dyn AnisetteHeadersProvider>,
    pub provider_type: AnisetteHeadersProviderType,
}

impl AnisetteHeadersProviderRes {
    pub fn local(provider: Box<dyn AnisetteHeadersProvider>) -> AnisetteHeadersProviderRes {
        AnisetteHeadersProviderRes {
            provider,
            provider_type: AnisetteHeadersProviderType::Local,
        }
    }

    pub fn remote(provider: Box<dyn AnisetteHeadersProvider>) -> AnisetteHeadersProviderRes {
        AnisetteHeadersProviderRes {
            provider,
            provider_type: AnisetteHeadersProviderType::Remote,
        }
    }
}

impl AnisetteHeaders {
    pub fn get_anisette_headers_provider(
        configuration: AnisetteConfiguration,
    ) -> Result<AnisetteHeadersProviderRes, AnisetteError> {
        #[cfg(feature = "remote-anisette-v3")]
        if let Some(url) = configuration.anisette_url_v3 {
            let url = remote_anisette_v3::provider_url(&url)?;
            log::info!(
                "Using remote Anisette at {}",
                url.origin().ascii_serialization()
            );
            return Ok(AnisetteHeadersProviderRes::remote(Box::new(
                remote_anisette_v3::RemoteAnisetteProviderV3::new(
                    url.to_string(),
                    configuration.configuration_path,
                    configuration.macos_serial,
                ),
            )));
        }

        log::info!("Using local StoreServicesCore Anisette");
        Self::get_ssc_anisette_headers_provider(configuration)
    }

    pub fn get_ssc_anisette_headers_provider(
        configuration: AnisetteConfiguration,
    ) -> Result<AnisetteHeadersProviderRes, AnisetteError> {
        let mut ssc_adi_proxy = store_services_core::StoreServicesCoreADIProxy::new(
            configuration.configuration_path(),
        )?;
        let config_path = configuration.configuration_path();
        ssc_adi_proxy.set_provisioning_path(config_path.to_str().ok_or(
            AnisetteError::InvalidArgument("configuration.configuration_path".to_string()),
        )?)?;
        Ok(AnisetteHeadersProviderRes::local(Box::new(
            ADIProxyAnisetteProvider::new(ssc_adi_proxy, config_path.to_path_buf())?,
        )))
    }
}
