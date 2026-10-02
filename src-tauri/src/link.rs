use serde::Serialize;
use url::{Host, Url};

const OPENABLE_SCHEMES: [&str; 3] = ["http", "https", "file"];

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LinkView {
    pub href: String,
    pub scheme: String,
    pub separator: String,
    pub userinfo: Option<String>,
    pub subdomain: Option<String>,
    pub domain: Option<String>,
    pub port: Option<u16>,
    pub path: String,
    pub query: Option<String>,
    pub fragment: Option<String>,
    pub unicode_host: Option<String>,
    pub openable: bool,
    pub signals: Vec<&'static str>,
}

pub struct ParsedLink {
    pub view: LinkView,
    pub host: Option<String>,
}

pub fn parse(raw: &str) -> Option<ParsedLink> {
    let url = Url::parse(raw.trim()).ok()?;
    let scheme = url.scheme().to_string();
    let mut signals = Vec::new();

    if scheme == "http" {
        signals.push("insecure");
    }

    let userinfo = match (url.username(), url.password()) {
        ("", None) => None,
        (user, None) => Some(user.to_string()),
        (user, Some(pass)) => Some(format!("{user}:{pass}")),
    };
    if userinfo.is_some() {
        signals.push("credentials");
    }

    let (subdomain, domain, host, unicode_host) = match url.host() {
        Some(Host::Domain(name)) => {
            let name = name.to_ascii_lowercase();
            let registrable = psl::domain_str(&name).unwrap_or(&name).to_string();
            let sub = name
                .strip_suffix(&registrable)
                .map(|s| s.trim_end_matches('.').to_string())
                .filter(|s| !s.is_empty());
            let unicode = unicode_form(&name);
            if unicode.is_some() {
                signals.push("punycode");
            }
            (sub, Some(registrable), Some(name), unicode)
        }
        Some(Host::Ipv4(ip)) => {
            signals.push("ip");
            (None, Some(ip.to_string()), None, None)
        }
        Some(Host::Ipv6(ip)) => {
            signals.push("ip");
            (None, Some(format!("[{ip}]")), None, None)
        }
        None => (None, None, None, None),
    };

    if url.port().is_some() {
        signals.push("port");
    }

    let separator = if url.has_host() || scheme == "file" { "://" } else { ":" }.to_string();

    let view = LinkView {
        href: url.as_str().to_string(),
        openable: OPENABLE_SCHEMES.contains(&scheme.as_str()),
        scheme,
        separator,
        userinfo,
        subdomain,
        domain,
        port: url.port(),
        path: url.path().to_string(),
        query: url.query().map(str::to_string),
        fragment: url.fragment().map(str::to_string),
        unicode_host,
        signals,
    };
    Some(ParsedLink { view, host })
}

fn unicode_form(ascii: &str) -> Option<String> {
    if !ascii.split('.').any(|label| label.starts_with("xn--")) {
        return None;
    }
    let (unicode, result) = idna::domain_to_unicode(ascii);
    result.ok()?;
    (unicode != ascii).then_some(unicode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_subdomain_from_registrable_domain() {
        let parsed = parse("https://login.accounts.example.co.uk/a?b=c#d").unwrap();
        assert_eq!(parsed.view.subdomain.as_deref(), Some("login.accounts"));
        assert_eq!(parsed.view.domain.as_deref(), Some("example.co.uk"));
        assert_eq!(parsed.view.query.as_deref(), Some("b=c"));
        assert_eq!(parsed.view.fragment.as_deref(), Some("d"));
    }

    #[test]
    fn flags_credentials_and_punycode() {
        let parsed = parse("http://paypal.com@xn--pypal-4ve.com/").unwrap();
        assert!(parsed.view.signals.contains(&"credentials"));
        assert!(parsed.view.signals.contains(&"punycode"));
        assert!(parsed.view.signals.contains(&"insecure"));
    }

    #[test]
    fn keeps_the_host_of_network_file_links() {
        let parsed = parse("file://wsl.localhost/Ubuntu/tmp/a%20b.pdf").unwrap();
        assert_eq!(parsed.view.href, "file://wsl.localhost/Ubuntu/tmp/a%20b.pdf");
    }

    #[test]
    fn rejects_non_urls() {
        assert!(parse("not a link").is_none());
    }
}
