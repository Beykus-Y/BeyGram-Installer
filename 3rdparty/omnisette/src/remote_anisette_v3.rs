// Implementing the SideStore Anisette v3 protocol

use std::{collections::HashMap, fs, io::Cursor, path::PathBuf};

use crate::adi_proxy::{LOOKUP_CLIENT_INFO, ProvisioningError};
use crate::transport::{
    APPLE_LOOKUP_URL, dictionary_at, https_url, initialize_tls, parse_dictionary, provisioning_url,
    string_at,
};
use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose;
use chrono::{DateTime, SubsecRound, Utc};
use futures_util::{SinkExt, stream::StreamExt};
use log::debug;
use plist::{Data, Dictionary};
use reqwest::{Client, ClientBuilder, RequestBuilder, Response, Url};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::fmt::Write;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

use crate::{AnisetteError, anisette_headers_provider::AnisetteHeadersProvider};

fn plist_to_string<T: serde::Serialize>(value: &T) -> Result<String, plist::Error> {
    plist_to_buf(value).map(|val| String::from_utf8(val).unwrap())
}

fn plist_to_buf<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, plist::Error> {
    let mut buf: Vec<u8> = Vec::new();
    let writer = Cursor::new(&mut buf);
    plist::to_writer_xml(writer, &value)?;
    Ok(buf)
}

fn bin_serialize<S>(x: &[u8], s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_bytes(x)
}

fn bin_serialize_opt<S>(x: &Option<Vec<u8>>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    x.clone().map(|i| Data::new(i)).serialize(s)
}

fn bin_deserialize_opt<'de, D>(d: D) -> Result<Option<Vec<u8>>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<Data> = Deserialize::deserialize(d)?;
    Ok(s.map(|i| i.into()))
}

fn bin_deserialize_16<'de, D>(d: D) -> Result<[u8; 16], D::Error>
where
    D: Deserializer<'de>,
{
    let s: Data = Deserialize::deserialize(d)?;
    let s: Vec<u8> = s.into();
    s.try_into()
        .map_err(|_| serde::de::Error::custom("Invalid provisioning identifier length"))
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        write!(&mut s, "{:02x}", b).unwrap();
    }
    s
}
fn base64_encode(data: &[u8]) -> String {
    general_purpose::STANDARD.encode(data)
}

#[derive(Deserialize)]
struct AnisetteClientInfo {
    client_info: String,
    user_agent: String,
}

#[derive(Serialize, Deserialize)]
pub struct AnisetteState {
    #[serde(
        serialize_with = "bin_serialize",
        deserialize_with = "bin_deserialize_16"
    )]
    keychain_identifier: [u8; 16],
    #[serde(
        serialize_with = "bin_serialize_opt",
        deserialize_with = "bin_deserialize_opt"
    )]
    adi_pb: Option<Vec<u8>>,
}

impl Default for AnisetteState {
    fn default() -> Self {
        AnisetteState {
            keychain_identifier: rand::random::<[u8; 16]>(),
            adi_pb: None,
        }
    }
}

impl AnisetteState {
    pub fn new() -> AnisetteState {
        AnisetteState::default()
    }

    pub fn is_provisioned(&self) -> bool {
        self.adi_pb.is_some()
    }

    fn md_lu(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.keychain_identifier);
        hasher.finalize().into()
    }

    fn device_id(&self) -> String {
        Uuid::from_bytes(self.keychain_identifier).to_string()
    }
}
pub struct AnisetteClient {
    client_info: AnisetteClientInfo,
    url: Url,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct ProvisionBodyData {
    header: Dictionary,
    request: Dictionary,
}

#[derive(Debug)]
pub struct AnisetteData {
    machine_id: String,
    one_time_password: String,
    routing_info: String,
    device_description: String,
    local_user_id: String,
    device_unique_identifier: String,
}

impl AnisetteData {
    pub fn get_headers(&self, serial: String) -> HashMap<String, String> {
        let dt: DateTime<Utc> = Utc::now().round_subsecs(0);

        HashMap::from_iter(
            [
                (
                    "X-Apple-I-Client-Time".to_string(),
                    dt.format("%+").to_string().replace("+00:00", "Z"),
                ),
                ("X-Apple-I-SRL-NO".to_string(), serial),
                ("X-Apple-I-TimeZone".to_string(), "UTC".to_string()),
                ("X-Apple-Locale".to_string(), "en_US".to_string()),
                ("X-Apple-I-MD-RINFO".to_string(), self.routing_info.clone()),
                ("X-Apple-I-MD-LU".to_string(), self.local_user_id.clone()),
                (
                    "X-Mme-Device-Id".to_string(),
                    self.device_unique_identifier.clone(),
                ),
                ("X-Apple-I-MD".to_string(), self.one_time_password.clone()),
                ("X-Apple-I-MD-M".to_string(), self.machine_id.clone()),
                (
                    "X-Mme-Client-Info".to_string(),
                    self.device_description.clone(),
                ),
            ]
            .into_iter(),
        )
    }
}

fn http_builder() -> ClientBuilder {
    initialize_tls();
    ClientBuilder::new()
        .http1_title_case_headers()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
}

fn make_reqwest() -> Result<Client, AnisetteError> {
    Ok(http_builder().build()?)
}

pub(crate) fn provider_url(value: &str) -> Result<Url, AnisetteError> {
    https_url(value)
        .filter(|url| url.query().is_none())
        .ok_or_else(|| {
            AnisetteError::InvalidArgument(
                "Anisette provider must be an HTTPS URL without credentials, query or fragment"
                    .into(),
            )
        })
}

fn checked(response: Response) -> Result<Response, ProvisioningError> {
    if response.status().is_success() {
        Ok(response)
    } else {
        Err(ProvisioningError::HttpStatus(response.status().as_u16()))
    }
}

impl AnisetteClient {
    pub async fn new(url: String) -> Result<AnisetteClient, AnisetteError> {
        let url = provider_url(&url)?;
        let path = format!("{}/v3/client_info", url.as_str().trim_end_matches('/'));
        let client_info = checked(make_reqwest()?.get(path).send().await?)?
            .json::<AnisetteClientInfo>()
            .await?;
        Ok(AnisetteClient { client_info, url })
    }

    fn build_apple_request(
        &self,
        state: &AnisetteState,
        builder: RequestBuilder,
        client_info: &str,
    ) -> RequestBuilder {
        let dt: DateTime<Utc> = Utc::now().round_subsecs(0);

        builder
            .header("X-Mme-Client-Info", client_info)
            .header("User-Agent", &self.client_info.user_agent)
            .header("Content-Type", "text/x-xml-plist")
            .header("X-Apple-I-MD-LU", encode_hex(&state.md_lu()))
            .header("X-Mme-Device-Id", state.device_id())
            .header("X-Apple-I-Client-Time", dt.format("%+").to_string())
            .header("X-Apple-I-TimeZone", "UTC")
            .header("X-Apple-Locale", "en_US")
    }

    pub async fn get_headers(&self, state: &AnisetteState) -> Result<AnisetteData, AnisetteError> {
        let path = format!("{}/v3/get_headers", self.url.as_str().trim_end_matches('/'));
        let http_client = make_reqwest()?;

        #[derive(Serialize)]
        struct GetHeadersBody {
            identifier: String,
            adi_pb: String,
        }
        let body = GetHeadersBody {
            identifier: base64_encode(&state.keychain_identifier),
            adi_pb: base64_encode(
                state
                    .adi_pb
                    .as_ref()
                    .ok_or(AnisetteError::AnisetteNotProvisioned)?,
            ),
        };

        #[derive(Deserialize)]
        #[serde(tag = "result")]
        enum AnisetteHeaders {
            GetHeadersError {
                message: String,
            },
            Headers {
                #[serde(rename = "X-Apple-I-MD-M")]
                machine_id: String,
                #[serde(rename = "X-Apple-I-MD")]
                one_time_password: String,
                #[serde(rename = "X-Apple-I-MD-RINFO")]
                routing_info: String,
            },
        }

        let headers = checked(http_client.post(path).json(&body).send().await?)?
            .json::<AnisetteHeaders>()
            .await?;
        match headers {
            AnisetteHeaders::GetHeadersError { message } => {
                if message.contains("-45061") {
                    Err(AnisetteError::AnisetteNotProvisioned)
                } else {
                    Err(ProvisioningError::InvalidResponse.into())
                }
            }
            AnisetteHeaders::Headers {
                machine_id,
                one_time_password,
                routing_info,
            } => Ok(AnisetteData {
                machine_id,
                one_time_password,
                routing_info,
                device_description: self.client_info.client_info.clone(),
                local_user_id: encode_hex(&state.md_lu()),
                device_unique_identifier: state.device_id(),
            }),
        }
    }

    pub async fn provision(&self, state: &mut AnisetteState) -> Result<(), AnisetteError> {
        debug!("Provisioning Anisette");
        let http_client = make_reqwest()?;
        let resp = self
            .build_apple_request(
                &state,
                http_client.get(APPLE_LOOKUP_URL),
                LOOKUP_CLIENT_INFO,
            )
            .send()
            .await?;
        let lookup = parse_dictionary(&checked(resp)?.bytes().await?)?;
        let start_provisioning_url = provisioning_url(&lookup, "midStartProvisioning")?;
        let end_provisioning_url = provisioning_url(&lookup, "midFinishProvisioning")?;

        let mut provision_ws_url = self.url.clone();
        let path = format!(
            "{}/v3/provisioning_session",
            provision_ws_url.path().trim_end_matches('/')
        );
        provision_ws_url.set_path(&path);
        provision_ws_url
            .set_scheme("wss")
            .map_err(|_| ProvisioningError::UntrustedEndpoint)?;
        let (mut connection, _) = connect_async(provision_ws_url.as_str()).await?;

        #[derive(Deserialize)]
        #[serde(tag = "result")]
        enum ProvisionInput {
            GiveIdentifier,
            GiveStartProvisioningData,
            GiveEndProvisioningData {
                #[allow(dead_code)] // it's not even dead, rust just has problems
                cpim: String,
            },
            ProvisioningSuccess {
                #[allow(dead_code)] // it's not even dead, rust just has problems
                adi_pb: String,
            },
        }

        while let Some(data) = connection.next().await {
            let data = data?;
            if data.is_text() {
                let txt = data.to_text()?;
                let msg: ProvisionInput = serde_json::from_str(txt)?;
                match msg {
                    ProvisionInput::GiveIdentifier => {
                        #[derive(Serialize)]
                        struct Identifier {
                            identifier: String, // base64
                        }
                        let identifier = Identifier {
                            identifier: base64_encode(&state.keychain_identifier),
                        };
                        connection
                            .send(Message::Text(serde_json::to_string(&identifier)?.into()))
                            .await?;
                    }
                    ProvisionInput::GiveStartProvisioningData => {
                        let http_client = make_reqwest()?;
                        let body_data = ProvisionBodyData {
                            header: Dictionary::new(),
                            request: Dictionary::new(),
                        };
                        let resp = self
                            .build_apple_request(
                                state,
                                http_client.post(start_provisioning_url.clone()),
                                &self.client_info.client_info,
                            )
                            .body(plist_to_string(&body_data)?)
                            .send()
                            .await?;
                        let protocol_val = parse_dictionary(&checked(resp)?.bytes().await?)?;
                        let spim = string_at(dictionary_at(&protocol_val, "Response")?, "spim")?;

                        debug!("GiveStartProvisioningData");
                        #[derive(Serialize)]
                        struct Spim {
                            spim: String, // base64
                        }
                        let spim = Spim {
                            spim: spim.to_string(),
                        };
                        connection
                            .send(Message::Text(serde_json::to_string(&spim)?.into()))
                            .await?;
                    }
                    ProvisionInput::GiveEndProvisioningData { cpim } => {
                        let http_client = make_reqwest()?;
                        let body_data = ProvisionBodyData {
                            header: Dictionary::new(),
                            request: Dictionary::from_iter([("cpim", cpim)].into_iter()),
                        };
                        let resp = self
                            .build_apple_request(
                                state,
                                http_client.post(end_provisioning_url.clone()),
                                &self.client_info.client_info,
                            )
                            .body(plist_to_string(&body_data)?)
                            .send()
                            .await?;
                        let protocol_val = parse_dictionary(&checked(resp)?.bytes().await?)?;
                        let response = dictionary_at(&protocol_val, "Response")?;

                        debug!("GiveEndProvisioningData");

                        #[derive(Serialize)]
                        struct EndProvisioning<'t> {
                            ptm: &'t str,
                            tk: &'t str,
                        }
                        let end_provisioning = EndProvisioning {
                            ptm: string_at(response, "ptm")?,
                            tk: string_at(response, "tk")?,
                        };
                        connection
                            .send(Message::Text(
                                serde_json::to_string(&end_provisioning)?.into(),
                            ))
                            .await?;
                    }
                    ProvisionInput::ProvisioningSuccess { adi_pb } => {
                        debug!("ProvisioningSuccess");
                        let adi_pb = general_purpose::STANDARD
                            .decode(adi_pb.trim())
                            .map_err(|_| ProvisioningError::InvalidResponse)?;
                        if adi_pb.is_empty() {
                            return Err(ProvisioningError::InvalidResponse.into());
                        }
                        state.adi_pb = Some(adi_pb);
                        connection.close(None).await?;
                        return Ok(());
                    }
                }
            } else if data.is_close() {
                break;
            }
        }

        Err(ProvisioningError::InvalidResponse.into())
    }
}

pub struct RemoteAnisetteProviderV3 {
    client_url: String,
    client: Option<AnisetteClient>,
    pub state: Option<AnisetteState>,
    configuration_path: PathBuf,
    serial: String,
}

impl RemoteAnisetteProviderV3 {
    pub fn new(
        url: String,
        configuration_path: PathBuf,
        serial: String,
    ) -> RemoteAnisetteProviderV3 {
        RemoteAnisetteProviderV3 {
            client_url: url,
            client: None,
            state: None,
            configuration_path,
            serial,
        }
    }
}

#[async_trait]
impl AnisetteHeadersProvider for RemoteAnisetteProviderV3 {
    async fn get_anisette_headers(
        &mut self,
        _skip_provisioning: bool,
    ) -> Result<HashMap<String, String>, AnisetteError> {
        if self.client.is_none() {
            self.client = Some(AnisetteClient::new(self.client_url.clone()).await?);
        }
        let client = self.client.as_ref().unwrap();

        fs::create_dir_all(&self.configuration_path)?;

        // A newly selected service must never receive another service's ADI blob.
        let provider_id = encode_hex(&Sha256::digest(client.url.as_str().as_bytes()));
        let config_path = self
            .configuration_path
            .join(format!("state-{provider_id}.plist"));
        if self.state.is_none() {
            self.state = Some(if let Ok(text) = plist::from_file(&config_path) {
                text
            } else {
                AnisetteState::new()
            });
        }

        let state = self.state.as_mut().unwrap();
        if !state.is_provisioned() {
            client.provision(state).await?;
            plist::to_file_xml(&config_path, state)?;
        }
        let data = match client.get_headers(&state).await {
            Ok(data) => data,
            Err(err) => {
                if matches!(err, AnisetteError::AnisetteNotProvisioned) {
                    state.adi_pb = None;
                    client.provision(state).await?;
                    plist::to_file_xml(config_path, state)?;
                    client.get_headers(&state).await?
                } else {
                    return Err(err);
                }
            }
        };
        Ok(data.get_headers(self.serial.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
        time::Duration,
    };

    fn serve_tls(response: Option<&'static [u8]>) -> (String, Vec<u8>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = format!("127.0.0.1:{}", listener.local_addr().unwrap().port());
        let cert = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).unwrap();
        let der = cert.serialize_der().unwrap();
        let config = rustls::ServerConfig::builder_with_provider(
            rustls::crypto::ring::default_provider().into(),
        )
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from(der.clone())],
            PrivatePkcs8KeyDer::from(cert.serialize_private_key_der()).into(),
        )
        .unwrap();
        let server = thread::spawn(move || {
            let (socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let connection = rustls::ServerConnection::new(config.into()).unwrap();
            let mut tls = rustls::StreamOwned::new(connection, socket);
            let mut request = Vec::new();
            let mut byte = [0];
            while !request.ends_with(b"\r\n\r\n") {
                match tls.read_exact(&mut byte) {
                    Ok(()) => request.push(byte[0]),
                    Err(_) if response.is_none() => return,
                    Err(err) => panic!("Synthetic HTTPS request failed: {err}"),
                }
            }
            tls.write_all(response.expect("untrusted TLS delivered an HTTP request"))
                .unwrap();
            tls.flush().unwrap();
        });
        (address, der, server)
    }

    #[tokio::test]
    async fn untrusted_certificate_is_rejected_by_https_and_websocket() {
        let (address, _, server) = serve_tls(None);
        let error = make_reqwest()
            .unwrap()
            .get(format!("https://{address}"))
            .send()
            .await
            .unwrap_err();
        assert!(error.is_connect());
        server.join().unwrap();

        let (address, _, server) = serve_tls(None);
        let error = connect_async(format!("wss://{address}")).await.unwrap_err();
        assert!(
            matches!(error, tokio_tungstenite::tungstenite::Error::Io(ref error)
            if error.kind() == std::io::ErrorKind::InvalidData)
        );
        server.join().unwrap();
    }

    #[tokio::test]
    async fn trusted_https_redirect_is_returned_without_following_it() {
        let (address, cert, server) = serve_tls(Some(
            b"HTTP/1.1 302 Found\r\nLocation: https://127.0.0.1:1/leak\r\nContent-Length: 0\r\n\r\n",
        ));
        let client = http_builder()
            .no_proxy()
            .tls_certs_only([reqwest::Certificate::from_der(&cert).unwrap()])
            .build()
            .unwrap();
        let response = client
            .get(format!("https://{address}"))
            .send()
            .await
            .unwrap();
        assert!(matches!(
            checked(response),
            Err(ProvisioningError::HttpStatus(302))
        ));
        server.join().unwrap();
    }
}
