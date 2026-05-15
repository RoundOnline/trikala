//! `trikala use <version>` — pin or switch the foundation version.
//!
//! Per U13, version pinning lives in `trikala.toml`. This command
//! edits that file in place (rustup-style) so a project compiled
//! today still builds in two years even if `trikala` itself moves on.

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// trikala foundation version, e.g. `0.1.0-alpha.1` or `0.2`.
    pub version: String,
}

pub fn run(args: Args, dry_run: bool) -> Result<()> {
    println!("pinning trikala foundation to `{}`", args.version);
    if dry_run {
        println!("(dry-run: trikala.toml unchanged)");
        return Ok(());
    }
    anyhow::bail!("pin rewrite not yet wired — coming in 0.1.0-alpha.2");
}
