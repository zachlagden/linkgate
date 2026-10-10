use std::collections::HashMap;

use sha2::{Digest, Sha256};

pub fn parse(text: &str) -> HashMap<String, String> {
    let mut sums = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        let Some((hash, rest)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let name = rest.trim().trim_start_matches('*');
        let valid = hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit());
        if valid && !name.is_empty() {
            sums.insert(name.to_string(), hash.to_ascii_lowercase());
        }
    }
    sums
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

pub fn parse_digest(name: &str, value: &str) -> Result<String, String> {
    let hex = value
        .trim()
        .split_once(':')
        .filter(|(algorithm, _)| algorithm.eq_ignore_ascii_case("sha256"))
        .map(|(_, hex)| hex)
        .filter(|hex| hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()));
    hex.map(str::to_ascii_lowercase)
        .ok_or_else(|| format!("GitHub published an unreadable checksum for {name}, so it can't be installed."))
}

pub fn lookup(name: &str, sums: &HashMap<String, String>) -> Result<String, String> {
    sums.get(name)
        .cloned()
        .ok_or_else(|| format!("The release's SHA256SUMS has no entry for {name}, so the download can't be checked."))
}

pub fn verify(name: &str, bytes: &[u8], expected: &str) -> Result<(), String> {
    if sha256_hex(bytes) == expected {
        Ok(())
    } else {
        Err(format!(
            "The downloaded {name} doesn't match its published checksum, so it was not installed."
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    #[test]
    fn parses_text_and_binary_mode_lines() {
        let text = format!("{EMPTY}  linkgate.exe\n{}  *linkgate-setup.exe\r\n\nnot a line\n", EMPTY.to_uppercase());
        let sums = parse(&text);
        assert_eq!(sums.get("linkgate.exe").map(String::as_str), Some(EMPTY));
        assert_eq!(sums.get("linkgate-setup.exe").map(String::as_str), Some(EMPTY));
        assert_eq!(sums.len(), 2);
    }

    #[test]
    fn ignores_malformed_hashes() {
        assert!(parse("abc  linkgate.exe\n").is_empty());
    }

    #[test]
    fn hashes_empty_input() {
        assert_eq!(sha256_hex(b""), EMPTY);
    }

    #[test]
    fn verify_accepts_a_match_and_rejects_everything_else() {
        assert!(verify("linkgate.exe", b"", EMPTY).is_ok());
        assert!(verify("linkgate.exe", b"tampered", EMPTY).is_err());
    }

    #[test]
    fn lookup_reads_the_sums_file_entry() {
        let sums = parse(&format!("{EMPTY}  linkgate.exe\n"));
        assert_eq!(lookup("linkgate.exe", &sums).unwrap(), EMPTY);
        assert!(lookup("other.exe", &sums).is_err());
    }

    #[test]
    fn parses_github_digests() {
        assert_eq!(parse_digest("linkgate.exe", &format!("sha256:{EMPTY}")).unwrap(), EMPTY);
        assert_eq!(parse_digest("linkgate.exe", &format!("SHA256:{}", EMPTY.to_uppercase())).unwrap(), EMPTY);
    }

    #[test]
    fn rejects_malformed_digests() {
        for bad in ["", "sha256:", "sha256:abc", EMPTY, &format!("sha1:{EMPTY}"), &format!("sha256:{}zz", &EMPTY[..62])] {
            let error = parse_digest("linkgate.exe", bad).unwrap_err();
            assert!(error.contains("unreadable checksum"), "{bad}: {error}");
        }
    }
}
