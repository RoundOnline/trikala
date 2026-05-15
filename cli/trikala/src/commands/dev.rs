//! `trikala dev` — phase ปัจจุบัน. Hot-reload local development.
//!
//! Per U2 this must boot in under 5 seconds on a mid-tier laptop and
//! must have a working file watcher attached by the time the window
//! appears. Per F11 the full dev loop (hot reload → debug panel →
//! save defaults → preserved state) is wired here.

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Override the project root.
    #[arg(long, default_value = ".")]
    pub root: String,

    /// Skip Rust hot-patch attempts; rebuild + restart on .rs changes.
    #[arg(long)]
    pub no_hot_rust: bool,
}

pub fn run(args: Args, dry_run: bool) -> Result<()> {
    println!("ปัจจุบัน · starting dev server in `{}`", args.root);
    if dry_run {
        println!("(dry-run: no watcher started)");
        return Ok(());
    }
    anyhow::bail!("dev loop not yet wired — coming in 0.1.0-alpha.2");
}
