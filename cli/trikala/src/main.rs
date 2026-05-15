//! `trikala` — born, built, shipped.
//!
//! See [`docs/trikala-axioms-v1.md`] for the contracts every command
//! is held to. The CLI surface is part of the public ABI per U5 (the
//! `--help` text alone must be sufficient documentation).

use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;

/// Born, built, shipped.
///
/// The three phases of making a game — scaffold, iterate, release —
/// in one CLI. Anonymous-first deploy: `trikala deploy` works without
/// signup. `trikala claim` upgrades the URL to permanent when ready.
#[derive(Debug, Parser)]
#[command(
    name = "trikala",
    version,
    about = "Born, built, shipped — a Rust + wgpu game pipeline.",
    long_about = None,
    propagate_version = true,
)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Print what would happen without doing it. (axiom U8)
    #[arg(long, global = true)]
    dry_run: bool,

    /// Verbose diagnostic output, including underlying traces. (U4)
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// อดีต · Scaffold a new project from a template.
    New(commands::new::Args),

    /// ปัจจุบัน · Run the game locally with hot reload.
    Dev(commands::dev::Args),

    /// ปัจจุบัน · Cross-platform build.
    Build(commands::build::Args),

    /// อนาคต · Deploy to round.online (anonymous-first) or another target.
    Deploy(commands::deploy::Args),

    /// อนาคต · Attach an anonymous deploy to your GitHub identity to make
    /// the URL permanent.
    Claim(commands::claim::Args),

    /// Diagnose toolchain, GPU, and project setup.
    Doctor(commands::doctor::Args),

    /// Pin or switch the trikala foundation version this project uses.
    Use(commands::use_::Args),

    /// Update the trikala CLI itself (uses the installer script under
    /// the hood; bypasses `cargo install` slowness — see axiom U11).
    Upgrade(commands::upgrade::Args),
}

fn main() -> Result<()> {
    // Strip the leading `cargo trikala` if invoked as a cargo subcommand.
    let mut args: Vec<_> = std::env::args().collect();
    if args.get(1).map(|s| s == "trikala").unwrap_or(false)
        && args[0].ends_with("cargo-trikala")
    {
        args.remove(1);
    }

    let cli = Cli::parse_from(args);
    init_tracing(cli.verbose);

    match cli.command {
        Command::New(a) => commands::new::run(a, cli.dry_run),
        Command::Dev(a) => commands::dev::run(a, cli.dry_run),
        Command::Build(a) => commands::build::run(a, cli.dry_run),
        Command::Deploy(a) => commands::deploy::run(a, cli.dry_run),
        Command::Claim(a) => commands::claim::run(a, cli.dry_run),
        Command::Doctor(a) => commands::doctor::run(a, cli.dry_run),
        Command::Use(a) => commands::use_::run(a, cli.dry_run),
        Command::Upgrade(a) => commands::upgrade::run(a, cli.dry_run),
    }
}

fn init_tracing(verbose: bool) {
    let level = if verbose { "trikala=debug" } else { "trikala=info" };
    // Lazy init — silent if RUST_LOG is set by user (axiom U9 — quiet on success).
    let _ = std::env::var("RUST_LOG").or_else::<std::env::VarError, _>(|_| {
        std::env::set_var("RUST_LOG", level);
        Ok(level.to_string())
    });
}
