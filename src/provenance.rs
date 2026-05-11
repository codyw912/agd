use crate::git;
use anyhow::Result;
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::path::Path;

pub fn patch_sha256(repo: &Path, base: &str, tip: &str) -> Result<String> {
    let patch = git::stdout_bytes(
        repo,
        [
            "diff",
            "--binary",
            "--full-index",
            "--no-ext-diff",
            base,
            tip,
        ],
    )?;
    Ok(hex_sha256(&patch))
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut hex, "{byte:02x}").expect("write to string");
    }
    hex
}
