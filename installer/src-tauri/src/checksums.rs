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

pub fn verify(name: &str, bytes: &[u8], sums: &HashMap<String, String>) -> Result<(), String> {
    let expected = sums
        .get(name)
        .ok_or_else(|| format!("SHA256SUMS has no entry for {name}, so the download can't be checked."))?;
    let actual = sha256_hex(bytes);
    if &actual == expected {
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
        let sums = parse(&format!("{EMPTY}  linkgate.exe\n"));
        assert!(verify("linkgate.exe", b"", &sums).is_ok());
        assert!(verify("linkgate.exe", b"tampered", &sums).is_err());
        assert!(verify("other.exe", b"", &sums).is_err());
    }
}
