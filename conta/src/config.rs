//! Conta Configuration
use anyhow::Result;
use core::str::FromStr;
use serde::{Deserialize, Serialize};
use std::{env, fs, path::Path};
use toml_edit::Document;

/// Conta configuration.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    /// Workspace members to skip when publishing.
    #[serde(default)]
    pub ignore: Vec<String>,
}

impl Config {
    /// Create a new configuration from path.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        toml::from_str(&fs::read_to_string(path)?).map_err(|e| e.into())
    }

    /// Create a new configuration from cargo manifest.
    ///
    /// A missing `[workspace.metadata.conta]` table yields the default
    /// (empty) config — conta's whole point is to publish everything,
    /// so no metadata means no exclusions.
    pub fn from_manifest(manifest: impl AsRef<Path>) -> Result<Self> {
        let doc = Document::from_str(&fs::read_to_string(manifest)?)?;
        Ok(doc["workspace"]["metadata"]["conta"]
            .as_table()
            .map(|t| toml::from_str::<Self>(&t.to_string()))
            .transpose()?
            .unwrap_or_default())
    }

    /// Create a new configuration from optional path.
    pub fn from_optional(path: Option<impl AsRef<Path>>) -> Result<Self> {
        if let Some(path) = path {
            if path.as_ref().exists() {
                return Self::from_path(path);
            }
        }

        let cwd = env::current_dir()?;
        let conta = cwd.join("Conta.toml");
        if conta.exists() {
            Self::from_path(conta)
        } else {
            Self::from_manifest(cwd.join("Cargo.toml"))
        }
    }
}

#[test]
fn from_manifest() -> Result<()> {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let config = Config::from_manifest(format!("{manifest}/../Cargo.toml"))?;
    assert!(config.ignore.is_empty());
    Ok(())
}
