//! `trikala claim` — upgrade an anonymous URL to a permanent one.
//!
//! Per D5, anonymous-first deploy is the unfair advantage. Claim is
//! the lazy-auth gate: GitHub OAuth happens *only* when the user
//! decides their game is worth keeping.

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// URL or anonymous slug returned by a previous `trikala deploy`.
    pub url: Option<String>,
}

pub fn run(args: Args, dry_run: bool) -> Result<()> {
    let url = args.url.as_deref().unwrap_or("<auto-detect from .trikala/last-deploy>");
    println!("claiming `{url}` — will open browser for GitHub OAuth");
    if dry_run {
        println!("(dry-run: no browser opened, no token exchanged)");
        return Ok(());
    }
    anyhow::bail!("claim flow not yet wired — coming in 0.2.0");
}
