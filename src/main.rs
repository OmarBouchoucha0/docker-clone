use anyhow::Error;
use clap::Parser;
mod runtime;
use runtime::run_container;

#[derive(Parser)]
#[command(name = "container")]
#[command(about = "A simple container runtime")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    Run {
        rootfs: String,
        command: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            rootfs,
            command,
            args,
        } => {
            run_container(&rootfs, &command, args)?;
        }
    }

    Ok(())
}
