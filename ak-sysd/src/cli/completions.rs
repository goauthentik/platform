use clap::CommandFactory;
use clap_complete::{Shell, generate};
use std::io;

use eyre::Result;

use crate::SysdArgs;

pub async fn completions(shell: Shell) -> Result<()> {
    generate(
        shell,
        &mut SysdArgs::command(),
        "ak-sysd",
        &mut io::stdout(),
    );
    Ok(())
}
