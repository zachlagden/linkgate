use std::io::Write;
use std::path::Path;

const MAGIC: &[u8; 8] = b"LGIX0001";
const HEADER_LEN: usize = 16;

pub fn parse_hosts(text: &str) -> Vec<String> {
    let mut domains: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_whitespace().last())
        .map(|domain| domain.trim_end_matches('.').to_ascii_lowercase())
        .filter(|domain| domain.contains('.') && domain != "0.0.0.0" && domain != "127.0.0.1")
        .collect();
    domains.sort_unstable();
    domains.dedup();
    domains
}

pub fn write(path: &Path, domains: &[String]) -> std::io::Result<()> {
    let mut offsets = Vec::with_capacity(domains.len() + 1);
    let mut cursor: u32 = 0;
    offsets.push(cursor);
    for domain in domains {
        cursor += domain.len() as u32;
        offsets.push(cursor);
    }
    let mut out = std::io::BufWriter::new(std::fs::File::create(path)?);
    out.write_all(MAGIC)?;
    out.write_all(&(domains.len() as u64).to_le_bytes())?;
    for offset in &offsets {
        out.write_all(&offset.to_le_bytes())?;
    }
    for domain in domains {
        out.write_all(domain.as_bytes())?;
    }
    out.flush()
}

pub struct Index<'a> {
    count: usize,
    offsets: &'a [u8],
    blob: &'a [u8],
}

impl<'a> Index<'a> {
    pub fn open(data: &'a [u8]) -> Option<Self> {
        if data.len() < HEADER_LEN || &data[..8] != MAGIC {
            return None;
        }
        let count = u64::from_le_bytes(data[8..16].try_into().ok()?) as usize;
        let blob_start = HEADER_LEN.checked_add(count.checked_add(1)?.checked_mul(4)?)?;
        if data.len() < blob_start {
            return None;
        }
        Some(Self {
            count,
            offsets: &data[HEADER_LEN..blob_start],
            blob: &data[blob_start..],
        })
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.count
    }

    fn offset(&self, i: usize) -> usize {
        let at = i * 4;
        u32::from_le_bytes([
            self.offsets[at],
            self.offsets[at + 1],
            self.offsets[at + 2],
            self.offsets[at + 3],
        ]) as usize
    }

    fn entry(&self, i: usize) -> &'a [u8] {
        let start = self.offset(i).min(self.blob.len());
        let end = self.offset(i + 1).min(self.blob.len());
        &self.blob[start..end.max(start)]
    }

    pub fn contains(&self, domain: &str) -> bool {
        let needle = domain.as_bytes();
        let (mut low, mut high) = (0, self.count);
        while low < high {
            let mid = low + (high - low) / 2;
            match self.entry(mid).cmp(needle) {
                std::cmp::Ordering::Less => low = mid + 1,
                std::cmp::Ordering::Greater => high = mid,
                std::cmp::Ordering::Equal => return true,
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_hosts_file() {
        let text = "# header\n0.0.0.0 b.example.com\n0.0.0.0 a.example.com\n\n0.0.0.0 B.example.com\n";
        let domains = parse_hosts(text);
        assert_eq!(domains, vec!["a.example.com", "b.example.com"]);
        let path = std::env::temp_dir().join("linkgate-index-test.idx");
        write(&path, &domains).unwrap();
        let data = std::fs::read(&path).unwrap();
        let index = Index::open(&data).unwrap();
        assert_eq!(index.len(), 2);
        assert!(index.contains("a.example.com"));
        assert!(index.contains("b.example.com"));
        assert!(!index.contains("c.example.com"));
        let _ = std::fs::remove_file(path);
    }
}
