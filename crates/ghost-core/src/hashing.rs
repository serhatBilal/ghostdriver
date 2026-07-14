//! Streaming SHA-256 hashing utilities.

use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use thiserror::Error;

/// Computes the lowercase SHA-256 digest of a file without loading it at once.
pub fn sha256_file(path: impl AsRef<Path>) -> Result<String, HashingError> {
    let path = path.as_ref();
    let file = File::open(path).map_err(|source| HashingError::Open {
        path: path.to_path_buf(),
        source,
    })?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|source| HashingError::Read {
                path: path.to_path_buf(),
                source,
            })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Failure to open or hash an artifact.
#[derive(Debug, Error)]
pub enum HashingError {
    /// The artifact could not be opened.
    #[error("failed to open {path}: {source}")]
    Open {
        /// Artifact path.
        path: PathBuf,
        /// I/O error.
        #[source]
        source: io::Error,
    },
    /// Reading the artifact failed.
    #[error("failed to read {path}: {source}")]
    Read {
        /// Artifact path.
        path: PathBuf,
        /// I/O error.
        #[source]
        source: io::Error,
    },
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn hashes_file_as_sha256() {
        let path =
            std::env::temp_dir().join(format!("ghostdriver-hash-fixture-{}", std::process::id()));
        fs::write(&path, b"abc").unwrap();
        let digest = sha256_file(&path).unwrap();
        fs::remove_file(path).unwrap();
        assert_eq!(
            digest,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
