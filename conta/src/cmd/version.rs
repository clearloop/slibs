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
    /// `[workspace.dependencies]` that is a workspace path dep — i.e.
    /// has both `path` and `version` fields. External crates (no `path`)
    /// and internal-only path deps (no `version`) are left alone.
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

        bump_path_dep_versions(&mut workspace, &version);

        fs::write(manifest, workspace.to_string())?;
        Ok(())
    }
}

/// Rewrite the `version` field on every `[workspace.dependencies]`
/// entry that carries both `path` and `version` — i.e. a workspace
/// path dep that is also published. External crates (no `path`) and
/// internal-only path deps (no `version`) are left alone.
fn bump_path_dep_versions(doc: &mut Document, version: &str) {
    let Some(deps) = doc["workspace"]["dependencies"].as_table_mut() else {
        return;
    };
    for (_name, item) in deps.iter_mut() {
        let Some(table) = item.as_table_like_mut() else {
            continue;
        };
        if table.contains_key("path") && table.contains_key("version") {
            table.insert("version", toml_edit::value(version.to_string()));
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bump_leaves_external_deps_alone() {
        // Regression: a previous cut only checked for the `version` key,
        // which clobbered external inline-table deps like reqwest and
        // serde. The filter must also require `path`.
        let input = r#"
[workspace.dependencies]
anyhow = "1.0.76"
reqwest = { version = "0.11.23", default-features = false }
serde = { version = "1.0.193", default-features = false }
ccli = { path = "ccli", version = "0.0.1" }
"#;
        let mut doc = Document::from_str(input).unwrap();
        bump_path_dep_versions(&mut doc, "0.0.2");
        let out = doc.to_string();
        assert!(out.contains(r#"anyhow = "1.0.76""#));
        assert!(out.contains(r#"reqwest = { version = "0.11.23", default-features = false }"#));
        assert!(out.contains(r#"serde = { version = "1.0.193", default-features = false }"#));
        assert!(out.contains(r#"ccli = { path = "ccli", version = "0.0.2" }"#));
    }

    #[test]
    fn bump_handles_subtable_form() {
        // The dotted-table form `[workspace.dependencies.foo]` parses
        // as a regular Table, not an InlineTable. `as_table_like_mut`
        // must still see it.
        let input = r#"
[workspace.dependencies.foo]
path = "foo"
version = "0.0.1"
"#;
        let mut doc = Document::from_str(input).unwrap();
        bump_path_dep_versions(&mut doc, "0.0.2");
        let out = doc.to_string();
        assert!(out.contains(r#"version = "0.0.2""#));
    }
}
