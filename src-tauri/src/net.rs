use std::sync::Arc;
use std::time::Duration;

pub fn agent(total_timeout: Duration) -> Result<ureq::Agent, String> {
    let tls = native_tls::TlsConnector::new().map_err(|e| e.to_string())?;
    Ok(ureq::AgentBuilder::new()
        .tls_connector(Arc::new(tls))
        .timeout_connect(Duration::from_secs(15))
        .timeout(total_timeout)
        .user_agent(concat!("linkgate/", env!("CARGO_PKG_VERSION")))
        .build())
}
