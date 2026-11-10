use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "rssh",
    version,
    about = "Session-scoped SSH wrapper with file upload"
)]
pub struct Cli {
    /// Path to configuration file
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Option<Command>,

    /// SSH host alias or destination (shorthand for `rssh connect <host>`)
    pub host: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Connect to a remote host through an interactive SSH session
    Connect {
        /// SSH host alias or destination
        host: String,

        /// Enable automatic drag-and-drop detection
        #[arg(long, default_value_t = false)]
        auto_drop: bool,

        /// Injection profile (literal, shell-posix, claude, codex)
        #[arg(long, default_value = "literal")]
        profile: String,
    },

    /// Check local and remote environment for compatibility
    Doctor {
        /// Optional host to check remote capabilities
        host: Option<String>,
    },

    /// Show status of an active session
    Status {
        /// Session identifier
        #[arg(long)]
        session: String,
    },

    /// Clean up remote upload artifacts
    Gc {
        /// SSH host alias
        host: String,

        /// Show what would be removed without removing it
        #[arg(long, default_value_t = true)]
        dry_run: bool,

        /// Actually remove artifacts
        #[arg(long, default_value_t = false)]
        apply: bool,
    },
}

impl Cli {
    /// Resolve the effective command — bare `rssh <host>` becomes `connect <host>`.
    pub fn resolved_command(self) -> ResolvedCommand {
        match self.command {
            Some(cmd) => ResolvedCommand::Subcommand(cmd),
            None => match self.host {
                Some(host) => ResolvedCommand::Subcommand(Command::Connect {
                    host,
                    auto_drop: false,
                    profile: "literal".into(),
                }),
                None => ResolvedCommand::NoArgs,
            },
        }
    }
}

pub enum ResolvedCommand {
    Subcommand(Command),
    NoArgs,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_parses_connect() {
        let cli = Cli::try_parse_from(["rssh", "connect", "myserver"]).unwrap();
        match cli.command {
            Some(Command::Connect {
                ref host,
                auto_drop,
                ..
            }) => {
                assert_eq!(host, "myserver");
                assert!(!auto_drop);
            }
            _ => panic!("expected Connect"),
        }
    }

    #[test]
    fn cli_shorthand_host() {
        let cli = Cli::try_parse_from(["rssh", "myserver"]).unwrap();
        assert!(cli.command.is_none());
        assert_eq!(cli.host.as_deref(), Some("myserver"));

        match cli.resolved_command() {
            ResolvedCommand::Subcommand(Command::Connect {
                ref host,
                auto_drop,
                ..
            }) => {
                assert_eq!(host, "myserver");
                assert!(!auto_drop);
            }
            _ => panic!("expected Connect from shorthand"),
        }
    }

    #[test]
    fn cli_parses_connect_with_flags() {
        let cli = Cli::try_parse_from([
            "rssh",
            "connect",
            "myserver",
            "--auto-drop",
            "--profile",
            "claude",
        ])
        .unwrap();
        match cli.command {
            Some(Command::Connect {
                ref host,
                auto_drop,
                ref profile,
            }) => {
                assert_eq!(host, "myserver");
                assert!(auto_drop);
                assert_eq!(profile, "claude");
            }
            _ => panic!("expected Connect"),
        }
    }

    #[test]
    fn cli_parses_doctor() {
        let cli = Cli::try_parse_from(["rssh", "doctor"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Doctor { host: None })));
    }

    #[test]
    fn cli_parses_doctor_with_host() {
        let cli = Cli::try_parse_from(["rssh", "doctor", "myserver"]).unwrap();
        match cli.command {
            Some(Command::Doctor { host: Some(ref h) }) => assert_eq!(h, "myserver"),
            _ => panic!("expected Doctor with host"),
        }
    }

    #[test]
    fn cli_no_args_is_no_args() {
        let cli = Cli::try_parse_from(["rssh"]).unwrap();
        assert!(matches!(cli.resolved_command(), ResolvedCommand::NoArgs));
    }

    #[test]
    fn verify_cli_structure() {
        Cli::command().debug_assert();
    }
}
