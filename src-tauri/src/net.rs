use std::time::Duration;

use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

pub fn agent(total_timeout: Duration) -> ureq::Agent {
    let tls = TlsConfig::builder()
        .provider(TlsProvider::NativeTls)
        .root_certs(RootCerts::PlatformVerifier)
        .build();
    ureq::Agent::config_builder()
        .tls_config(tls)
        .http_status_as_error(false)
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_global(Some(total_timeout))
        .user_agent(concat!("linkgate/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}
