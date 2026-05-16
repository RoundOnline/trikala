//! `trikala upgrade` — replace the trikala CLI binary in place.
//!
//! Per U11, self-update is first-class. The installer script at
//! `trikala.round.online/install.sh` is the source of truth; this command
//! re-invokes it with the desired version (latest by default).

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Version to upgrade to (default: latest).
    pub version: Option<String>,
}

pub fn run(args: Args, dry_run: bool) -> Result<()> {
    let target = args.version.as_deref().unwrap_or("latest");
    println!("upgrading trikala CLI to `{target}`");
    if dry_run {
        println!("(dry-run: nothing fetched, nothing replaced)");
        return Ok(());
    }
    anyhow::bail!("self-update not yet wired — coming in 0.1.0-alpha.2");
}
