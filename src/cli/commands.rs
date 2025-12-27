//! CLI subcommand definitions.

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Show status of all harnesses.
    Status,

    /// Initialize bridle configuration.
    Init,

    /// Manage profiles.
    #[command(subcommand)]
    Profile(ProfileCommands),

    /// Launch terminal UI.
    Tui,
}

#[derive(Subcommand, Debug)]
pub enum ProfileCommands {
    /// List profiles for a harness.
    List {
        /// Harness name (claude-code, opencode, goose).
        harness: String,
    },

    /// Show details of a specific profile.
    Show {
        /// Harness name.
        harness: String,
        /// Profile name.
        name: String,
    },

    /// Create a new profile.
    Create {
        /// Harness name.
        harness: String,
        /// Profile name.
        name: String,
    },

    /// Delete a profile.
    Delete {
        /// Harness name.
        harness: String,
        /// Profile name.
        name: String,
    },

    /// Switch to a profile (set as active).
    Switch {
        /// Harness name.
        harness: String,
        /// Profile name.
        name: String,
    },
}
