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
    /// Run the server and the window together.
    Dev,
    /// Run the server only. The listen address comes from gasket.toml.
    Serve,
    /// Run the window only. It dials the client URL in gasket.toml.
    Ui,
}

fn main() {
    let cli = Cli::parse();
    if let Err(err) = run(cli) {
        eprintln!("gasket: {err}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let flag: &[&str] = match cli.command {
        Commands::Dev => &[],
        Commands::Serve => &["--serve"],
        Commands::Ui => &["--ui"],
    };
    let status = if let Some(package) = app_package() {
        Command::new("cargo")
            .arg("run")
            .arg("--manifest-path")
            .arg(package.join("Cargo.toml"))
            .arg("--")
            .args(flag)
            .status()?
    } else if let Some(root) = workspace_root() {
        Command::new("cargo")
            .args(["run", "-p", "gasket-hello", "--"])
            .args(flag)
            .current_dir(root)
            .status()?
    } else {
        return Err(
            "run gasket from an application with gasket.toml, or from the Gasket repository".into(),
        );
    };
    std::process::exit(status.code().unwrap_or(1));
}

fn app_package() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if dir.join("gasket.toml").is_file() && dir.join("Cargo.toml").is_file() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
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
