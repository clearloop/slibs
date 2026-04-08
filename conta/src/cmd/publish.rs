//! Command publish

use crate::{graph, version};
use anyhow::{anyhow, Result};
use ccli::clap::{self, Parser};
use core::str::FromStr;
use std::{path::PathBuf, process::Command};
use toml_edit::Document;

/// Publish crates.
#[derive(Debug, Parser, Clone)]
pub struct Publish;

impl Publish {
    /// Run publish.
    ///
    /// Walks the workspace in topological order, skipping crates listed
    /// in `ignore` and any whose current version is already on crates.io.
    pub fn run(&self, manifest: &PathBuf, ignore: &[String]) -> Result<()> {
        let workspace = Document::from_str(&std::fs::read_to_string(manifest)?)?;
        let version = workspace["workspace"]["package"]["version"]
            .as_str()
            .ok_or_else(|| anyhow!("Failed to parse version from workspace {manifest:?}"))?;

        let order = graph::resolve(manifest, ignore)?;

        for pkg in order {
            if version::verify(&pkg, version)? {
                println!("Package {pkg}@{version} has already been published.");
                continue;
            }

            if !self.publish(&pkg)? {
                return Err(anyhow!("Failed to publish {pkg}"));
            }
        }

        Ok(())
    }

    /// Publish cargo package
    fn publish(&self, package: &str) -> Result<bool> {
        Command::new("cargo")
            .arg("publish")
            .arg("-p")
            .arg(package)
            .arg("--allow-dirty")
            .status()
            .map(|status| status.success())
            .map_err(|err| err.into())
    }
}
