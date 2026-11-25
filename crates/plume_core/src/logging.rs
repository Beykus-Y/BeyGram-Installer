use std::io::Write;

use reqwest::header::HeaderValue;

use crate::Error;

/// Wire-level library logs can include request bodies, tokens and pairing keys.
fn allow_target(target: &str) -> bool {
    !matches!(
        target.split("::").next().unwrap_or(target),
        "dbus_secret_service"
            | "h2"
            | "hyper"
            | "hyper_util"
            | "idevice"
            | "reqwest"
            | "rustls"
            | "tokio_tungstenite"
            | "tungstenite"
    )
}

pub fn init_logging() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format(|buffer, record| {
            if allow_target(record.target()) {
                writeln!(
                    buffer,
                    "[{} {} {}] {}",
                    buffer.timestamp_millis(),
                    record.level(),
                    record.target(),
                    record.args()
                )?;
            }
            Ok(())
        })
        .init();
}

pub(crate) fn sensitive_header(value: &str) -> Result<HeaderValue, Error> {
    let mut header = HeaderValue::from_str(value).map_err(|_| Error::Parse)?;
    header.set_sensitive(true);
    Ok(header)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_logs_are_blocked_and_secret_header_debug_is_redacted() {
        assert!(!allow_target("idevice::remote_pairing::rp_pairing_file"));
        assert!(!allow_target("hyper::proto::h1"));
        assert!(allow_target("plume_core::developer"));

        let secret = "synthetic-header-secret";
        let header = sensitive_header(secret).unwrap();
        assert!(!format!("{header:?}").contains(secret));
    }
}
