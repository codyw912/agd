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

pub fn adopted_patch_sha256(repo: &Path, commit: &str) -> Result<String> {
    let parent_ref = format!("{commit}^");
    let parent = git::stdout(repo, ["rev-parse", parent_ref.as_str()])?;
    patch_sha256(repo, parent.trim(), commit)
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut hex, "{byte:02x}").expect("write to string");
    }
    hex
}
