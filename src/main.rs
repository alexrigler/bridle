mod cli;
mod config;
mod error;
mod harness;
mod tui;

use clap::Parser;
use cli::{Commands, ConfigCommands, ProfileCommands};

#[derive(Parser)]
#[command(name = "bridle")]
#[command(version, about = "Unified AI harness configuration manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Status => cli::status::display_status(),
        Commands::Init => cli::init::run_init(),
        Commands::Profile(profile_cmd) => match profile_cmd {
            ProfileCommands::List { harness } => cli::profile::list_profiles(&harness),
            ProfileCommands::Show { harness, name } => cli::profile::show_profile(&harness, &name),
            ProfileCommands::Create {
                harness,
                name,
                from_current,
            } => {
                if from_current {
                    cli::profile::create_profile_from_current(&harness, &name)
                } else {
                    cli::profile::create_profile(&harness, &name)
                }
            }
            ProfileCommands::Delete { harness, name } => {
                cli::profile::delete_profile(&harness, &name)
            }
            ProfileCommands::Switch { harness, name } => {
                cli::profile::switch_profile(&harness, &name)
            }
            ProfileCommands::Edit { harness, name } => cli::profile::edit_profile(&harness, &name),
            ProfileCommands::Diff {
                harness,
                name,
                other,
            } => cli::profile::diff_profiles(&harness, &name, other.as_deref()),
        },
        Commands::Tui => cli::tui::run_tui()?,
        Commands::Config(config_cmd) => match config_cmd {
            ConfigCommands::Set { key, value } => cli::config_cmd::set_config(&key, &value),
            ConfigCommands::Get { key } => cli::config_cmd::get_config(&key),
        },
    }

    Ok(())
}
