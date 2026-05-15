//! `trikala deploy` — phase อนาคต. Ship the game.
//!
//! Per D5 this is anonymous-first: with no target argument the command
//! pushes to round.online and prints back an ephemeral URL valid for
//! 7 days (D6). `trikala claim` upgrades the URL to permanent. Per U7
//! there are no interactive prompts during a deploy.

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Deploy target. Default = round.online (anonymous-first).
    #[arg(default_value = "round")]
    pub target: String,

    /// Variant to deploy. Defaults to `release`.
    #[arg(long, default_value = "release")]
    pub variant: String,
}

pub fn run(args: Args, dry_run: bool) -> Result<()> {
    println!("อนาคต · deploying variant `{}` to `{}`", args.variant, args.target);
    if dry_run {
        match args.target.as_str() {
            "round" | "round.online" => println!(
                "(dry-run: would upload to round.online; URL would be ephemeral for 7 days)"
            ),
            "itch"       => println!("(dry-run: would invoke butler with TRIKALA_ITCH_TOKEN)"),
            "steam"      => println!("(dry-run: would invoke steamcmd)"),
            "cloudflare" => println!("(dry-run: would invoke wrangler with CF_API_TOKEN)"),
            other        => println!("(dry-run: unknown target `{other}`)"),
        }
        return Ok(());
    }
    anyhow::bail!("deploy not yet wired — coming in 0.1.0-alpha.2");
}
