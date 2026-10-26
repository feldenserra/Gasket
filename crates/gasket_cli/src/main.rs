use std::path::PathBuf;
use std::process::Command;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "gasket")]
#[command(about = "Gasket developer CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the reference app in this repository.
    Dev {
        /// Port the reference app binds
        #[arg(short, long, default_value_t = 3000)]
        port: u16,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(err) = run(cli) {
        eprintln!("gasket: {err}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Dev { port } => {
            let root = workspace_root().ok_or("run gasket dev from the Gasket repository")?;
            let status = Command::new("cargo")
                .args([
                    "run",
                    "-p",
                    "gasket-hello",
                    "--",
                    "--port",
                    &port.to_string(),
                ])
                .current_dir(root)
                .status()?;
            std::process::exit(status.code().unwrap_or(1));
        }
    }
}

fn workspace_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if dir.join("examples/hello/Cargo.toml").is_file() && dir.join("Cargo.toml").is_file() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}
