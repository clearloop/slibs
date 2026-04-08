pub use crate::{
    cmd::{publish::Publish, version::Version},
    Config,
};
use anyhow::Result;
use ccli::{
    clap::{self, Parser},
    App,
};
use std::path::PathBuf;

mod publish;
mod version;

/// Commands of this tool.
#[derive(Debug, Parser, Clone)]
pub enum Command {
    Version(Version),
    Publish(Publish),
}

/// Modern tool for bumping crate versions and
/// publishing them.
#[derive(Debug, Parser, Clone)]
pub struct Conta {
    /// The path of the cargo manifest, if not provided, the
    /// current directory is used.
    #[clap(short, long)]
    manifest: Option<PathBuf>,

    /// The path of `Conta.toml`
    #[clap(short, long)]
    config: Option<PathBuf>,

    /// The command to run.
    #[clap(subcommand)]
    command: Command,
}

impl Conta {
    /// Get the manifest path.
    pub fn manifest(&self) -> PathBuf {
        if let Some(p) = &self.manifest {
            p.into()
        } else {
            PathBuf::from("Cargo.toml")
        }
    }

    /// Parse the config from the input path.
    pub fn config(&self) -> Result<Config> {
        Config::from_optional(self.config.as_deref())
    }
}

impl App for Conta {
    fn verbose(&self) -> u8 {
        0
    }

    fn run(&self) -> Result<()> {
        let manifest = self.manifest();

        match &self.command {
            Command::Version(version) => version.run(&manifest),
            Command::Publish(publish) => publish.run(&manifest, &self.config()?.ignore),
        }
    }
}
