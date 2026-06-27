use crate::manifest::{sha256_file, Manifest};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub struct BackupResult {
    pub manifest: Manifest,
    pub output_path: PathBuf,
    pub output_size_bytes: u64,
    pub manifest_sha256: String,
}

pub fn physical_backup(
    data_dir: &Path,
    wal_path: Option<&Path>,
    output: &Path,
) -> Result<BackupResult, BackupError> {
    if !data_dir.exists() {
        return Err(BackupError::DataDirNotFound(data_dir.to_path_buf()));
    }
    let manifest = Manifest::scan(data_dir, wal_path).map_err(BackupError::Io)?;

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(BackupError::Io)?;
        }
    }

    let file = File::create(output).map_err(BackupError::Io)?;
    let mut gz = GzEncoder::new(file, Compression::default());
    let manifest_data = manifest_bytes(&manifest)?;
    write_tar_header(&mut gz, "manifest.json", manifest_data.len() as u64)?;
    gz.write_all(&manifest_data).map_err(BackupError::Io)?;
    write_padding(&mut gz, manifest_data.len())?;
    for entry in &manifest.data_files {
        let full = data_dir.join(&entry.path);
        let arc = format!("data/{}", entry.path);
        let bytes = fs::read(&full).map_err(BackupError::Io)?;
        write_tar_header(&mut gz, &arc, bytes.len() as u64)?;
        gz.write_all(&bytes).map_err(BackupError::Io)?;
        write_padding(&mut gz, bytes.len())?;
    }
    if let (Some(wp), Some(wf)) = (wal_path, manifest.wal_file.as_ref()) {
        if wp.exists() {
            let bytes = fs::read(wp).map_err(BackupError::Io)?;
            let verified = sha256_file(wp).map_err(BackupError::Io)?;
            if verified != wf.sha256 {
                return Err(BackupError::ChecksumMismatch {
                    path: wf.path.clone(),
                });
            }
            write_tar_header(&mut gz, "wal/sqlrustgo.wal", bytes.len() as u64)?;
            gz.write_all(&bytes).map_err(BackupError::Io)?;
            write_padding(&mut gz, bytes.len())?;
        }
    }
    gz.write_all(&[0u8; 1024]).map_err(BackupError::Io)?;
    let _gz = gz.finish().map_err(BackupError::Io)?;

    let output_size = fs::metadata(output).map_err(BackupError::Io)?.len();
    let manifest_json = serde_json::to_string(&manifest).map_err(BackupError::Json)?;
    let manifest_sha = crate::manifest::sha256_bytes(manifest_json.as_bytes());

    Ok(BackupResult {
        manifest,
        output_path: output.to_path_buf(),
        output_size_bytes: output_size,
        manifest_sha256: manifest_sha,
    })
}

fn manifest_bytes(m: &Manifest) -> Result<Vec<u8>, BackupError> {
    serde_json::to_vec_pretty(m).map_err(BackupError::Json)
}

fn write_tar_header<W: Write>(w: &mut W, name: &str, size: u64) -> Result<(), BackupError> {
    let mut h = [0u8; 512];
    let name_bytes = name.as_bytes();
    let copy_len = name_bytes.len().min(100);
    h[..copy_len].copy_from_slice(&name_bytes[..copy_len]);
    h[100..108].copy_from_slice(b"0000644\0");
    h[108..116].copy_from_slice(b"0000000\0");
    h[116..124].copy_from_slice(b"0000000\0");
    h[124..136].copy_from_slice(format!("{:011o}\0", size).as_bytes());
    h[136..148].copy_from_slice(b"00000000000\0");
    let chk: u32 = h.iter().map(|&b| b as u32).sum();
    let octal = format!("{:o}", chk);
    let mut chk_field = [b' '; 8];
    let pad_start = if octal.len() < 6 { 6 - octal.len() } else { 0 };
    let copy_len = octal.len().min(6);
    chk_field[pad_start..pad_start + copy_len].copy_from_slice(&octal.as_bytes()[..copy_len]);
    chk_field[6] = 0;
    chk_field[7] = b' ';
    h[148..156].copy_from_slice(&chk_field);
    h[257..265].copy_from_slice(b"ustar  \0");
    w.write_all(&h).map_err(BackupError::Io)?;
    Ok(())
}

#[allow(dead_code)] // reserved for tar archive finalization
fn write_tar_end_marker<W: Write>(w: &mut W) -> Result<(), BackupError> {
    let z = [0u8; 512];
    w.write_all(&z).map_err(BackupError::Io)?;
    Ok(())
}

fn write_padding<W: Write>(w: &mut W, size: usize) -> Result<(), BackupError> {
    let pad = (512 - (size % 512)) % 512;
    if pad > 0 {
        let zeros = vec![0u8; pad];
        w.write_all(&zeros).map_err(BackupError::Io)?;
    }
    Ok(())
}

pub fn tar_extract_all(input: &Path, out_dir: &Path) -> Result<Vec<String>, BackupError> {
    let compressed = fs::read(input).map_err(BackupError::Io)?;
    let bytes = decode_gzip_or_raw(&compressed)?;
    let mut entries = Vec::new();
    let mut offset = 0usize;
    while offset + 512 <= bytes.len() {
        if bytes[offset..offset + 512] == [0u8; 512][..] {
            break;
        }
        let header = &bytes[offset..offset + 512];
        let name_bytes = &header[..100];
        let name_end = name_bytes
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(name_bytes.len());
        let name = String::from_utf8_lossy(&name_bytes[..name_end]).to_string();
        let size_bytes = &header[124..136];
        let size_str = String::from_utf8_lossy(size_bytes).to_string();
        let size: u64 = u64::from_str_radix(size_str.trim_end_matches('\0').trim(), 8)
            .map_err(|_| BackupError::CorruptTar)?;
        let blocks = size.div_ceil(512);
        let data_start = offset + 512;
        let data_end = data_start + size as usize;
        if data_end > bytes.len() {
            return Err(BackupError::CorruptTar);
        }
        let data = &bytes[data_start..data_end];
        let out_path = out_dir.join(&name);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(BackupError::Io)?;
        }
        if name == "manifest.json" || name.starts_with("data/") || name.starts_with("wal/") {
            fs::write(&out_path, data).map_err(BackupError::Io)?;
        }
        entries.push(name);
        if blocks == 0 {
            offset = data_end + 512;
        } else {
            offset = data_start + (blocks as usize) * 512;
        }
    }
    Ok(entries)
}

pub fn tar_extract_one(input: &Path, name: &str) -> Result<Vec<u8>, BackupError> {
    let compressed = fs::read(input).map_err(BackupError::Io)?;
    let bytes = decode_gzip_or_raw(&compressed)?;
    let mut offset = 0usize;
    while offset + 512 <= bytes.len() {
        if bytes[offset..offset + 512] == [0u8; 512][..] {
            break;
        }
        let header = &bytes[offset..offset + 512];
        let name_bytes = &header[..100];
        let name_end = name_bytes
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(name_bytes.len());
        let entry_name = String::from_utf8_lossy(&name_bytes[..name_end]).to_string();
        let size_str = String::from_utf8_lossy(&header[124..136]).to_string();
        let size: u64 = u64::from_str_radix(size_str.trim_end_matches('\0').trim(), 8)
            .map_err(|_| BackupError::CorruptTar)?;
        let blocks = size.div_ceil(512);
        let data_start = offset + 512;
        let data_end = data_start + size as usize;
        if data_end > bytes.len() {
            return Err(BackupError::CorruptTar);
        }
        if entry_name == name {
            return Ok(bytes[data_start..data_end].to_vec());
        }
        if blocks == 0 {
            offset = data_end + 512;
        } else {
            offset = data_start + (blocks as usize) * 512;
        }
    }
    Err(BackupError::EntryNotFound(name.to_string()))
}

fn decode_gzip_or_raw(compressed: &[u8]) -> Result<Vec<u8>, BackupError> {
    if compressed.len() >= 2 && compressed[0] == 0x1f && compressed[1] == 0x8b {
        let mut decoder = GzDecoder::new(compressed);
        let mut out = Vec::new();
        decoder.read_to_end(&mut out).map_err(BackupError::Io)?;
        Ok(out)
    } else {
        Ok(compressed.to_vec())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("data directory not found: {0}")]
    DataDirNotFound(PathBuf),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("checksum mismatch: {path}")]
    ChecksumMismatch { path: String },
    #[error("corrupt tar archive")]
    CorruptTar,
    #[error("entry not found: {0}")]
    EntryNotFound(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_backup_empty() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("backup.tar.gz");
        let r = physical_backup(dir.path(), None, &out).unwrap();
        assert_eq!(r.manifest.data_files.len(), 0);
        assert!(r.output_size_bytes > 0);
        assert!(out.exists());
    }

    #[test]
    fn test_backup_with_data_and_wal() {
        let data_dir = TempDir::new().unwrap();
        let wal_dir = TempDir::new().unwrap();
        std::fs::write(data_dir.path().join("t1.json"), b"{}").unwrap();
        let wal = wal_dir.path().join("sqlrustgo.wal");
        std::fs::write(&wal, b"wal-bytes").unwrap();
        let out = data_dir.path().join("backup.tar.gz");
        let r = physical_backup(data_dir.path(), Some(&wal), &out).unwrap();
        assert_eq!(r.manifest.data_files.len(), 1);
        assert!(r.manifest.wal_file.is_some());
    }

    #[test]
    fn test_tar_extract_round_trip() {
        let data_dir = TempDir::new().unwrap();
        let wal_dir = TempDir::new().unwrap();
        std::fs::write(data_dir.path().join("a.json"), b"data-a").unwrap();
        let wal = wal_dir.path().join("sqlrustgo.wal");
        std::fs::write(&wal, b"wal-bytes-here").unwrap();
        let out = data_dir.path().join("backup.tar.gz");
        physical_backup(data_dir.path(), Some(&wal), &out).unwrap();

        let extract_dir = TempDir::new().unwrap();
        let entries = tar_extract_all(&out, extract_dir.path()).unwrap();
        assert!(entries.contains(&"manifest.json".to_string()));
        assert!(entries.iter().any(|e| e == "data/a.json"));
        assert!(entries.iter().any(|e| e == "wal/sqlrustgo.wal"));

        let manifest_path = extract_dir.path().join("manifest.json");
        let m = Manifest::read_from(&manifest_path).unwrap();
        assert_eq!(m.data_files.len(), 1);
    }

    #[test]
    fn test_tar_extract_one() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("x.json"), b"hello").unwrap();
        let out = dir.path().join("b.tar.gz");
        physical_backup(dir.path(), None, &out).unwrap();
        let data = tar_extract_one(&out, "data/x.json").unwrap();
        assert_eq!(data, b"hello");
    }

    #[test]
    fn test_backup_data_dir_not_found() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("b.tar.gz");
        let r = physical_backup(&dir.path().join("nonexistent"), None, &out);
        assert!(matches!(r, Err(BackupError::DataDirNotFound(_))));
    }

    #[test]
    fn test_tar_extract_corrupt_short() {
        let dir = TempDir::new().unwrap();
        let f = dir.path().join("corrupt.tar.gz");
        std::fs::write(&f, b"not a tar").unwrap();
        let r = tar_extract_all(&f, dir.path());
        assert!(r.is_ok());
        assert_eq!(r.unwrap().len(), 0);
    }
}
