use std::{fs, path::PathBuf, vec};

use apple_codesign::{
    SigningSettings,
    cryptography::{InMemoryPrivateKey, PrivateKey},
};
// TODO: why do we have pem and pem_rfc7468 deps again?
use pem_rfc7468::{LineEnding, encode_string};
use rand::rngs::OsRng;
use rcgen::{DnType, KeyPair, PKCS_RSA_SHA256};
use rsa::{
    RsaPrivateKey,
    pkcs1::EncodeRsaPublicKey,
    pkcs8::{DecodePrivateKey, EncodePrivateKey},
};
use x509_certificate::{CapturedX509Certificate, X509Certificate};

use crate::{
    Error,
    developer::{DeveloperSession, qh::certs::Cert},
};

pub(crate) const MACHINE_NAME: &str = "AltStore";

pub struct CertificateIdentity {
    pub cert: Option<CapturedX509Certificate>,
    pub key: Option<Box<dyn PrivateKey>>,
    pub machine_id: Option<String>,
    pub serial_number: Option<String>,
    pub p12_data: Option<Vec<u8>>,
    pub new: bool,
}

impl CertificateIdentity {
    // Use for cli context or if you actually store pems? why would you do that though
    pub async fn new_with_paths(paths: Option<Vec<PathBuf>>) -> Result<Self, Error> {
        let mut cert = Self {
            cert: None,
            key: None,
            machine_id: None,
            p12_data: None,
            serial_number: None,
            new: false,
        };

        if let Some(paths) = paths {
            for path in &paths {
                let pem_data = fs::read(path)?;
                cert.resolve_certificate_from_contents(pem_data)?;
            }
        }

        Ok(cert)
    }

    pub async fn new_with_session(
        session: &DeveloperSession,
        config_path: PathBuf,
        machine_name: Option<String>,
        team_id: &String,
        export_password: Option<&str>,
    ) -> Result<Self, Error> {
        let machine_name = machine_name.unwrap_or_else(|| MACHINE_NAME.to_string());

        let key_path = Self::key_dir(config_path, &team_id)?.join("key.pem");

        let mut identity = Self {
            cert: None,
            key: None,
            machine_id: None,
            p12_data: None,
            serial_number: None,
            new: false,
        };

        let existing_key = plume_secrets::read_or_migrate(&key_path, |bytes| {
            let pem = std::str::from_utf8(bytes).map_err(|_| Error::Parse)?;
            Ok::<_, Error>(RsaPrivateKey::from_pkcs8_pem(pem)?)
        })?;
        if existing_key.is_none() {
            plume_secrets::prepare(&key_path)?;
        }

        let certs = session.qh_list_certs(team_id).await?.certificates;
        let existing_identity = match existing_key {
            Some(key) => identity
                .find_certificate(certs, &key, &machine_name)
                .await?
                .map(|cert| (cert, key)),
            None => None,
        };
        let (certificate, private_key) = if let Some(existing) = existing_identity {
            existing
        } else {
            identity.new = true;
            identity
                .request_new_certificate(session, team_id, &machine_name)
                .await?
        };
        let cert_pem = encode_string(
            "CERTIFICATE",
            LineEnding::LF,
            certificate
                .cert_content
                .ok_or(Error::CertificatePemMissing)?
                .as_ref(),
        )
        .unwrap();
        let key_pem = private_key.to_pkcs8_pem(Default::default())?;
        if identity.new {
            plume_secrets::write(&key_path, key_pem.as_bytes())?;
        }
        let key_pair = [cert_pem.into_bytes(), key_pem.as_bytes().to_vec()];

        // TODO: this may be horrendious
        if let Some(password) = export_password {
            identity.p12_data =
                Some(Self::create_pkcs12(&key_pair, password).ok_or_else(|| {
                    Error::Certificate("Unable to create certificate export".into())
                })?);
        }

        for pem in key_pair {
            identity.resolve_certificate_from_contents(pem)?;
        }

        Ok(identity)
    }

    // <config_path>/keys/<team_id>
    fn key_dir(path: PathBuf, team_id: &String) -> Result<PathBuf, Error> {
        if team_id.is_empty() || !team_id.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
            return Err(Error::Certificate("Invalid Apple team identifier".into()));
        }
        let dir = path.join("keys").join(team_id);

        fs::create_dir_all(&dir)?;

        Ok(dir)
    }

    fn set_machine_id(&mut self, machine_id: String) {
        self.machine_id = Some(machine_id);
    }

    fn set_serial_number(&mut self, serial_number: String) {
        self.serial_number = Some(serial_number);
    }

    // TODO: cleanest p12 code of them all
    // the main horror about p12 creation is that we rely on p12-keystore which is
    // just another unnecessary dependency, but the p12 crate that applecodesign-rs
    // uses has no support for modern encryption, hopefully this doesn't add that
    // much more bloat
    fn create_pkcs12(data: &[Vec<u8>; 2], password: &str) -> Option<Vec<u8>> {
        if password.is_empty() {
            return None;
        }
        let cert_der = pem::parse(&data[0]).ok()?.contents().to_vec();
        let key_der = pem::parse(&data[1]).ok()?.contents().to_vec();

        let cert = p12_keystore::Certificate::from_der(&cert_der).ok()?;

        let local_key_id = {
            use sha1::{Digest, Sha1};
            let mut hasher = Sha1::new();
            hasher.update(&key_der);
            let hash = hasher.finalize();
            hash[..8].to_vec()
        };

        let key_chain = p12_keystore::PrivateKeyChain::new(key_der, local_key_id, vec![cert]);

        let mut keystore = p12_keystore::KeyStore::new();
        keystore.add_entry(
            "plume",
            p12_keystore::KeyStoreEntry::PrivateKeyChain(key_chain),
        );

        let writer = keystore.writer(password);
        writer.write().ok()
    }

    // applecodesign-rs needs our contents as strings to sign
    fn resolve_certificate_from_contents(&mut self, contents: Vec<u8>) -> Result<(), Error> {
        for pem in pem::parse_many(contents).map_err(Error::Pem)? {
            match pem.tag() {
                "CERTIFICATE" => {
                    self.cert = Some(CapturedX509Certificate::from_der(pem.contents())?);
                }
                "PRIVATE KEY" => {
                    self.key = Some(Box::new(InMemoryPrivateKey::from_pkcs8_der(
                        pem.contents(),
                    )?));
                }
                "RSA PRIVATE KEY" => {
                    self.key = Some(Box::new(InMemoryPrivateKey::from_pkcs1_der(
                        pem.contents(),
                    )?));
                }
                tag => log::debug!("(unhandled PEM tag {}; ignoring)", tag),
            }
        }

        Ok(())
    }
}

impl CertificateIdentity {
    async fn find_certificate(
        &mut self,
        certs: Vec<Cert>,
        priv_key: &RsaPrivateKey,
        machine_name: &str,
    ) -> Result<Option<Cert>, Error> {
        let pub_key_der_obj = priv_key.to_public_key().to_pkcs1_der()?.as_bytes().to_vec();

        for cert in certs {
            if cert.machine_name.as_deref() == Some(machine_name) {
                if let Some(cert_content) = &cert.cert_content {
                    let parsed_cert = X509Certificate::from_der(&cert_content)?;
                    if pub_key_der_obj == parsed_cert.public_key_data().as_ref() {
                        // We need to save the machine_id for our P12
                        if let Some(ref machine_id) = cert.machine_id {
                            self.set_machine_id(machine_id.clone());
                        }

                        self.set_serial_number(cert.serial_number.clone());

                        return Ok(Some(cert));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn request_new_certificate(
        &mut self,
        session: &DeveloperSession,
        team_id: &String,
        machine_name: &String,
    ) -> Result<(Cert, RsaPrivateKey), Error> {
        let priv_key = RsaPrivateKey::new(&mut OsRng, 2048)?;
        let priv_key_der = priv_key.to_pkcs8_der()?;
        let priv_key_pair = KeyPair::from_der(priv_key_der.as_bytes())?;

        let mut params = rcgen::CertificateParams::new(vec![]);
        params.alg = &PKCS_RSA_SHA256;
        params.key_pair = Some(priv_key_pair);

        let dn = &mut params.distinguished_name;
        dn.push(DnType::CountryName, "US");
        dn.push(DnType::StateOrProvinceName, "STATE");
        dn.push(DnType::LocalityName, "LOCAL");
        dn.push(DnType::OrganizationName, "ORGNIZATION");
        dn.push(DnType::CommonName, "CN");

        let cert_csr = rcgen::Certificate::from_params(params)?.serialize_request_pem()?;

        let cert_id = session
            .qh_submit_cert_csr(team_id, cert_csr, machine_name)
            .await
            .map_err(certificate_request_error)?
            .cert_request;

        // We need to save the machine_id for our P12
        if let Some(ref machine_id) = cert_id.machine_id {
            self.set_machine_id(machine_id.clone());
        }

        self.set_serial_number(cert_id.serial_num.clone());

        // We request again, and hope this has our new certificate
        // ready.... if not then woops... thats too bad isnt it
        let certs = session
            .qh_list_certs(&team_id)
            .await?
            .certificates
            .into_iter()
            .find(|c| c.certificate_id == cert_id.certificate_id);

        Ok((certs.ok_or(Error::CertificatePemMissing)?, priv_key))
    }
}

impl CertificateIdentity {
    pub fn load_into_signing_settings<'settings, 'slf: 'settings>(
        &'slf self,
        settings: &'settings mut SigningSettings<'slf>,
    ) -> Result<(), Error> {
        let signing_cert = self.cert.clone().ok_or(Error::CertificatePemMissing)?;
        let signing_key = self.key.as_ref().ok_or(Error::CertificatePemMissing)?;

        settings.set_signing_key(signing_key.as_key_info_signer(), signing_cert);
        settings.chain_apple_certificates();
        settings.set_team_id_from_signing_certificate();

        Ok(())
    }
}

fn certificate_request_error(error: Error) -> Error {
    match error {
        Error::DeveloperApi {
            result_code: 7460, ..
        } => Error::Certificate(
            "Apple development certificate quota reached. Ask the team owner to free a certificate slot in the Apple developer portal, then retry.".into(),
        ),
        error => error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quota_exhaustion_requires_team_owner_action() {
        let error = certificate_request_error(Error::DeveloperApi {
            url: String::new(),
            result_code: 7460,
            http_code: None,
            message: "synthetic response".into(),
        });

        assert!(matches!(error, Error::Certificate(ref message) if message.contains("team owner")));
    }
}
