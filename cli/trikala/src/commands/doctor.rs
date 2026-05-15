//! `trikala doctor` — preflight check.
//!
//! Per research, opaque native-prereq failures are Tauri's biggest UX
//! miss. `doctor` is trikala's defense: every common toolchain or GPU
//! issue produces a `code/cause/hint/docs_url` block per U10.

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Run extended GPU benchmark.
    #[arg(long)]
    pub gpu: bool,

    /// Generate a flamegraph of the current dev build (via `inferno`).
    #[arg(long)]
    pub flame: bool,
}

pub fn run(args: Args, _dry_run: bool) -> Result<()> {
    println!("trikala doctor — preflight");
    println!("  rust:    checking…");
    println!("  cargo:   checking…");
    println!("  wgpu:    checking…");
    println!("  network: checking…");
    if args.gpu   { println!("  gpu benchmark: requested"); }
    if args.flame { println!("  flamegraph: requested (will shell out to `inferno`)"); }
    println!("(checks not yet wired — coming in 0.1.0-alpha.2)");
    Ok(())
}
