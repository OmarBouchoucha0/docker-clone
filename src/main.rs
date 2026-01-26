use clap::Parser;

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
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { rootfs, command, args } => {
            println!("Running {} in {}", command, rootfs);
        }
    }
}
