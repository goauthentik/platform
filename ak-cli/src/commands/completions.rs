use clap::CommandFactory;
use clap_complete::{Shell, generate};
use std::io;

use crate::CliArgs;
use eyre::Result;

pub async fn completions(shell: Shell) -> Result<()> {
    generate(shell, &mut CliArgs::command(), "ak", &mut io::stdout());
    Ok(())
}
