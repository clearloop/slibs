//! Command bump
use anyhow::{anyhow, Result};
use ccli::clap::{self, Parser, ValueEnum};
use semver::Version as SemVer;
use std::{fs, path::PathBuf, str::FromStr};
use toml_edit::Document;

/// Bump versions.
#[derive(Debug, Parser, Clone)]
pub struct Version {
    /// The version to bump.
    bump: Bump,

    /// Dry run the command and print the result.
    #[clap(short, long, value_name = "dry-run")]
    dry_run: bool,
}

impl Version {
    /// Bumps the version to the given one.
    ///
    /// Updates `[workspace.package].version` and every entry in
    /// `[workspace.dependencies]` that already has a `version` field —
    /// i.e. workspace path deps that are also published. Entries without
    /// a version (internal-only path deps, external crates) are left
    /// alone.
    pub fn run(&self, manifest: &PathBuf) -> Result<()> {
        let mut workspace = Document::from_str(&std::fs::read_to_string(manifest)?)?;
        let bump = self.bump.run(
            workspace["workspace"]["package"]["version"]
                .as_str()
                .ok_or_else(|| anyhow!("No version found in [workspace.package]"))?,
        )?;

        let version = bump.to_string();
        workspace["workspace"]["package"]["version"] = toml_edit::value(version.clone());

        if self.dry_run {
            println!("{workspace}");
            return Ok(());
        }

        if let Some(deps) = workspace["workspace"]["dependencies"].as_table_mut() {
            for (_name, item) in deps.iter_mut() {
                let Some(table) = item.as_table_like_mut() else {
                    continue;
                };
                if table.contains_key("version") {
                    table.insert("version", toml_edit::value(version.clone()));
                }
            }
        }

        fs::write(manifest, workspace.to_string())?;
        Ok(())
    }
}

/// Version bumper
#[derive(Debug, Clone, ValueEnum)]
pub enum Bump {
    Patch,
    Minor,
    Major,
    #[value(name = "[semver]")]
    Semver,
    #[value(skip)]
    Version(SemVer),
}

impl Bump {
    /// Bumps the version.
    pub fn run(&self, version: &str) -> Result<SemVer> {
        let mut version = SemVer::parse(version)?;
        match self {
            Bump::Patch => version.patch += 1,
            Bump::Minor => version.minor += 1,
            Bump::Major => version.major += 1,
            Bump::Semver => {}
            Bump::Version(ver) => version = ver.clone(),
        }

        Ok(version)
    }
}

impl FromStr for Bump {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "patch" => Ok(Bump::Patch),
            "minor" => Ok(Bump::Minor),
            "major" => Ok(Bump::Major),
            _ => Ok(Bump::Version(SemVer::parse(s)?)),
        }
    }
}
