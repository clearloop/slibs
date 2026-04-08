//! Workspace dependency graph.
//!
//! Spawns `cargo metadata` to enumerate workspace members and their
//! inter-member dependencies, then topologically sorts them so that
//! `cargo publish` can walk them in a valid order.

use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    process::Command,
};

#[derive(Debug, Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Package {
    name: String,
    id: String,
    dependencies: Vec<Dependency>,
}

#[derive(Debug, Deserialize)]
struct Dependency {
    name: String,
    /// `"dev"`, `"build"`, or absent/null for a normal dep. Only normal
    /// deps gate publish order — dev-deps are stripped from the uploaded
    /// crate, and build-deps are irrelevant for workspace-internal ordering.
    #[serde(default)]
    kind: Option<String>,
}

/// Resolve workspace members in publish order, dropping any whose name
/// appears in `ignore`.
///
/// The returned list is a topological sort of the subgraph induced by
/// workspace members — i.e. a crate only appears after every workspace
/// crate it depends on. Ties are broken by name for determinism.
pub fn resolve(manifest: &Path, ignore: &[String]) -> Result<Vec<String>> {
    let output = Command::new("cargo")
        .arg("metadata")
        .arg("--no-deps")
        .arg("--format-version")
        .arg("1")
        .arg("--manifest-path")
        .arg(manifest)
        .output()?;

    if !output.status.success() {
        return Err(anyhow!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let meta: Metadata = serde_json::from_slice(&output.stdout)?;

    let members: BTreeSet<&str> = meta.workspace_members.iter().map(String::as_str).collect();
    let ignore: BTreeSet<&str> = ignore.iter().map(String::as_str).collect();

    // name -> package, restricted to workspace members
    let by_name: BTreeMap<&str, &Package> = meta
        .packages
        .iter()
        .filter(|p| members.contains(p.id.as_str()))
        .map(|p| (p.name.as_str(), p))
        .collect();

    // Build the full intra-workspace graph first. Dev-deps and build-deps
    // are dropped — they're stripped from the uploaded crate and don't
    // affect publish order. A legal A-tests-B / B-tests-A dev-dep loop
    // would otherwise look like a cycle.
    let mut graph: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (name, pkg) in &by_name {
        let deps: BTreeSet<&str> = pkg
            .dependencies
            .iter()
            .filter(|d| d.kind.is_none())
            .map(|d| d.name.as_str())
            .filter(|n| by_name.contains_key(n))
            .collect();
        graph.insert(*name, deps);
    }

    // Guard: an ignored crate with reverse deps in the publish set would
    // cause cargo to upload a manifest pointing at a version that was
    // never published. Fail loudly rather than ship a broken release.
    for (name, deps) in &graph {
        if ignore.contains(name) {
            continue;
        }
        for dep in deps {
            if ignore.contains(dep) {
                return Err(anyhow!(
                    "{name} depends on ignored crate {dep}; \
                     either un-ignore {dep} or remove it from {name}'s dependencies",
                ));
            }
        }
    }

    // Drop ignored nodes.
    graph.retain(|name, _| !ignore.contains(name));

    topo_sort(graph)
}

fn topo_sort<'a>(mut graph: BTreeMap<&'a str, BTreeSet<&'a str>>) -> Result<Vec<String>> {
    let mut order = Vec::with_capacity(graph.len());

    while !graph.is_empty() {
        // Collect nodes with no unresolved deps, sorted by name for stability.
        let ready: Vec<&str> = graph
            .iter()
            .filter(|(_, deps)| deps.is_empty())
            .map(|(name, _)| *name)
            .collect();

        if ready.is_empty() {
            return Err(anyhow!(
                "dependency cycle among workspace members: {:?}",
                graph.keys().collect::<Vec<_>>()
            ));
        }

        for name in &ready {
            graph.remove(name);
        }
        for deps in graph.values_mut() {
            for name in &ready {
                deps.remove(name);
            }
        }
        for name in ready {
            order.push(name.to_string());
        }
    }

    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_this_workspace() -> Result<()> {
        let manifest = format!("{}/../Cargo.toml", env!("CARGO_MANIFEST_DIR"));
        let order = resolve(Path::new(&manifest), &[])?;
        // ccli has no workspace deps; conta depends on ccli — so ccli must
        // come first.
        let ccli = order.iter().position(|n| n == "ccli");
        let conta = order.iter().position(|n| n == "conta");
        assert!(ccli.is_some() && conta.is_some());
        assert!(ccli < conta);
        Ok(())
    }

    #[test]
    fn ignore_excludes_leaf_crate() -> Result<()> {
        // `conta` is a sink in this workspace — nothing depends on it,
        // so ignoring it is safe.
        let manifest = format!("{}/../Cargo.toml", env!("CARGO_MANIFEST_DIR"));
        let order = resolve(Path::new(&manifest), &["conta".to_string()])?;
        assert!(!order.iter().any(|n| n == "conta"));
        assert!(order.iter().any(|n| n == "ccli"));
        Ok(())
    }

    #[test]
    fn ignoring_crate_with_reverse_deps_errors() {
        // `conta` depends on `ccli`; ignoring `ccli` must fail rather
        // than silently producing a broken publish plan.
        let manifest = format!("{}/../Cargo.toml", env!("CARGO_MANIFEST_DIR"));
        let err = resolve(Path::new(&manifest), &["ccli".to_string()]).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("conta"));
        assert!(msg.contains("ccli"));
    }
}
