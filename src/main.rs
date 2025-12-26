use anyhow::Result;
use clap::{Parser, Subcommand};

use workbloom::commands::{cleanup, setup};
use workbloom::output;

#[derive(Parser)]
#[command(
    author,
    version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_HASH"), ")"),
    about,
    long_about = None
)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Set up a new git worktree with automatic file copying", visible_alias = "s")]
    Setup {
        #[arg(help = "The branch name for the worktree")]
        branch_name: String,
        
        #[arg(long, help = "Skip starting a new shell (default is to start shell)")]
        no_shell: bool,

        #[arg(long, help = "Print only the worktree path to stdout (implies --no-shell)")]
        print_path: bool,
    },
    
    #[command(about = "Clean up worktrees", visible_alias = "c")]
    Cleanup {
        #[arg(long, conflicts_with_all = &["pattern", "interactive", "status"], help = "Remove only merged worktrees")]
        merged: bool,
        
        #[arg(long, value_name = "PATTERN", conflicts_with_all = &["merged", "interactive", "status"], help = "Remove worktrees matching pattern")]
        pattern: Option<String>,
        
        #[arg(long, conflicts_with_all = &["merged", "pattern", "status"], help = "Interactive removal")]
        interactive: bool,
        
        #[arg(long, conflicts_with_all = &["merged", "pattern", "interactive"], help = "Show merge status of all branches")]
        status: bool,

        #[arg(long, help = "Force cleanup without remote branch checks (use with --merged). Still protects recently created worktrees")]
        force: bool,
    },
}

fn main() -> Result<()> {
    colored::control::set_override(should_use_color());
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Setup { branch_name, no_shell, print_path } => {
            output::set_machine_output(print_path);
            let start_shell = !no_shell && !print_path;
            setup::execute(&branch_name, start_shell, print_path)?;
        }
        Commands::Cleanup {
            merged,
            pattern,
            interactive,
            status,
            force,
        } => {
            let mode = if merged || (pattern.is_none() && !interactive && !status) {
                cleanup::CleanupMode::Merged { force }
            } else if let Some(p) = pattern {
                cleanup::CleanupMode::Pattern(p)
            } else if interactive {
                cleanup::CleanupMode::Interactive
            } else {
                cleanup::CleanupMode::Status
            };
            
            cleanup::execute(mode)?;
        }
    }
    
    Ok(())
}

fn should_use_color() -> bool {
    if std::env::var("NO_COLOR").is_ok() || std::env::var("CLICOLOR").map(|v| v == "0").unwrap_or(false) {
        return false;
    }
    true
}
