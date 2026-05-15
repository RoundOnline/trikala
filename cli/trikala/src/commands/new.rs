//! `trikala new` — phase อดีต. Scaffold a new project.
//!
//! Per U1, this command must work with zero flags using a sensible
//! default template. Per F24, the template name resolves against a
//! hash-pinned registry — not hard-coded.

use anyhow::Result;
use clap::Args as ClapArgs;

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Project name. Becomes the directory name and the URL slug.
    pub name: String,

    /// Template to scaffold from. Defaults to `blank`.
    #[arg(short = 't', long, default_value = "blank")]
    pub template: String,

    /// Skip writing files — print what would happen.
    #[arg(long)]
    pub force: bool,
}

pub fn run(args: Args, dry_run: bool) -> Result<()> {
    println!("อดีต · scaffolding `{}` from template `{}`", args.name, args.template);
    if dry_run {
        println!("(dry-run: nothing written)");
        return Ok(());
    }
    // TODO: shell out to cargo-generate against the templates registry.
    anyhow::bail!("scaffolding not yet wired — coming in 0.1.0-alpha.2");
}
