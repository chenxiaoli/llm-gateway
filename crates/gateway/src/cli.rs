use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "llm-gateway", about = "LLM Gateway server and admin CLI")]
pub struct Cli {
    /// Path to config.toml. Defaults to ./config.toml. Used by the CLI
    /// subcommand when present; ignored when running the server (the server
    /// reads from the working directory by convention).
    #[arg(long, global = true, default_value = "config.toml")]
    pub config: String,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Grant or revoke platform_admin for a user. Operator escape hatch for
    /// bootstrap when the first-user auto-promotion is disabled.
    GrantPlatformAdmin {
        /// Username to grant/revoke. Must already exist in the users table.
        #[arg(long)]
        username: String,
        /// Revoke instead of grant. Sets platform_role = NULL.
        #[arg(long, default_value_t = false)]
        revoke: bool,
        /// Override the last-admin guard when revoking. Required to demote
        /// the only platform_admin. Prints a warning when used.
        #[arg(long, default_value_t = false)]
        allow_last_admin: bool,
    },
}