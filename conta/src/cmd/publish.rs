//! Command publish

use crate::{graph, version};
use anyhow::{anyhow, Result};
use ccli::clap::{self, Parser};
use core::str::FromStr;
use std::{path::PathBuf, process::Command};
use toml_edit::Document;

/// Publish crates.
#[derive(Debug, Parser, Clone)]
pub struct Publish {
    /// Print the resolved publish plan without invoking `cargo publish`.
    #[clap(short, long)]
    dry_run: bool,
}

impl Publish {
    /// Run publish.
    ///
    /// Walks the workspace in topological order, skipping crates listed
    /// in `ignore` and any whose current version is already on crates.io.
    /// With `--dry-run`, prints the plan and returns without publishing.
    pub fn run(&self, manifest: &PathBuf, ignore: &[String]) -> Result<()> {
        let workspace = Document::from_str(&std::fs::read_to_string(manifest)?)?;
        let version = workspace["workspace"]["package"]["version"]
            .as_str()
            .ok_or_else(|| anyhow!("Failed to parse version from workspace {manifest:?}"))?;

        let order = graph::resolve(manifest, ignore)?;

        let mut published = 0u32;
        let mut skipped = 0u32;
        let mut failed = Vec::new();

        for pkg in order {
            if version::verify(&pkg, version)? {
                println!("{pkg}@{version} already published, skipping");
                skipped += 1;
                continue;
            }

            if self.dry_run {
                println!("{pkg}@{version} would publish");
                continue;
            }

            if let Err(err) = self.publish(&pkg) {
                eprintln!("failed to publish {pkg}: {err}, continuing");
                failed.push(pkg);
            } else {
                published += 1;
            }
        }

        println!(
            "\nsummary: {published} published, {skipped} skipped, {} failed",
            failed.len(),
        );
        for pkg in &failed {
            println!("  failed: {pkg}");
        }

        if !failed.is_empty() {
            return Err(anyhow!("{} crate(s) failed to publish", failed.len()));
        }

        Ok(())
    }

    /// Publish cargo package
    fn publish(&self, package: &str) -> Result<()> {
        let status = Command::new("cargo")
            .arg("publish")
            .arg("-p")
            .arg(package)
            .arg("--allow-dirty")
            .status()?;
        if !status.success() {
            return Err(anyhow!("cargo publish -p {package} exited with {status}"));
        }
        Ok(())
    }
}
