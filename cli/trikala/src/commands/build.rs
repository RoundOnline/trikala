//! `trikala build` — cross-platform builds via variants.
//!
//! Per F14 variants live in `trikala.toml [variants.*]`. Per F17 each
//! variant produces a named binary `{project}-{variant}{.ext}`.

use anyhow::Result;
use clap::{Args as ClapArgs, ValueEnum};

#[derive(Debug, Clone, ValueEnum)]
pub enum Target {
    /// Native host platform (Win/Mac/Linux).
    Host,
    /// WebAssembly for the browser.
    Web,
    Android,
    Ios,
    All,
}

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Variant from `trikala.toml`. Defaults to `release`.
    #[arg(long, default_value = "release")]
    pub variant: String,

    /// Target platform to build for.
    #[arg(short, long, value_enum, default_value_t = Target::Host)]
    pub target: Target,
}

pub fn run(args: Args, dry_run: bool) -> Result<()> {
    println!("building variant `{}` for {:?}", args.variant, args.target);
    if dry_run {
        println!("(dry-run: no build invoked)");
        return Ok(());
    }
    anyhow::bail!("build not yet wired — coming in 0.1.0-alpha.2");
}
