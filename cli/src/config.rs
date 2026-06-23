use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct BucketConfig {
    pub bucket_id: String,
    pub client_token: String,
}

pub fn load_env_file(path: &PathBuf) -> Result<HashMap<String, String>> {
    let iter = dotenvy::from_path_iter(path)
        .with_context(|| format!("failed to read env file {}", path.display()))?;
    iter.collect::<Result<HashMap<_, _>, _>>()
        .with_context(|| format!("failed to parse env file {}", path.display()))
}

pub fn resolve_bucket(
    bucket_flag: Option<&str>,
    env_path: &PathBuf,
) -> Result<BucketConfig> {
    let file_vars = load_env_file(env_path).unwrap_or_default();
    let bucket_id = bucket_flag
        .map(str::to_string)
        .or_else(|| file_vars.get("ARGUS_BUCKET_ID").cloned())
        .context(
            "No bucket specified. Use --bucket or set ARGUS_BUCKET_ID in .env",
        )?;
    let client_token = file_vars
        .get("ARGUS_BUCKET_TOKEN")
        .or_else(|| file_vars.get("ARGUS_CLIENT_TOKEN"))
        .or_else(|| file_vars.get("ARGUS_TOKEN"))
        .cloned()
        .context("ARGUS_BUCKET_TOKEN (or ARGUS_CLIENT_TOKEN / ARGUS_TOKEN) not found in .env")?;
    Ok(BucketConfig {
        bucket_id,
        client_token,
    })
}
