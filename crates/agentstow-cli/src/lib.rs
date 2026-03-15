mod bootstrap;
mod cli;
mod commands;
mod link;
mod output;
mod workspace;

use clap::Parser;

pub use cli::Cli;

use bootstrap::{CommandContext, apply_cli_cwd};
use output::{emit_error, init_tracing};

pub async fn run() -> i32 {
    let cli = Cli::parse();
    let json = cli.json;

    let _ = init_tracing();

    if let Err(err) = apply_cli_cwd(cli.cwd.as_deref()) {
        emit_error(json, &err);
        return err.exit_code().as_i32();
    }

    let ctx = match CommandContext::from_cli(&cli) {
        Ok(ctx) => ctx,
        Err(err) => {
            emit_error(json, &err);
            return err.exit_code().as_i32();
        }
    };

    match commands::run_cli(cli.command, &ctx).await {
        Ok(()) => 0,
        Err(err) => {
            emit_error(json, &err);
            err.exit_code().as_i32()
        }
    }
}

#[cfg(test)]
mod tests;
