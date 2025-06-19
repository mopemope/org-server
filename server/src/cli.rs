use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Parse an Org-mode file and output JSON
    Parse {
        /// Path to the Org-mode file to parse
        file: PathBuf,
        /// Output file path (if not specified, outputs to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Pretty-print the JSON output
        #[arg(short, long)]
        pretty: bool,
        /// Include position information in the output
        #[arg(long)]
        include_position: bool,
        /// Include empty sections in the output
        #[arg(long)]
        include_empty_sections: bool,
        /// Maximum depth for nested sections (default: 10)
        #[arg(long, default_value = "10")]
        max_depth: usize,
    },
    /// Start the server mode for file monitoring and reminders
    Server {
        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,
        /// Port to bind the server to
        #[arg(short, long, default_value = "3000")]
        port: u16,
        /// Host to bind the server to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },
    /// Print help information
    Help {
        /// Subcommand to show help for
        #[arg(value_name = "SUBCOMMAND")]
        subcommand: Option<String>,
    },
}

impl Cli {
    pub fn get_command(&self) -> Commands {
        self.command.clone().unwrap_or(Commands::Server {
            config: None,
            port: 3000,
            host: "127.0.0.1".to_string(),
        })
    }

    /// Print help for a specific subcommand or general help
    pub fn print_help(subcommand: Option<&str>) {
        match subcommand {
            Some("parse") => {
                println!(
                    "Parse an Org-mode file and output JSON

USAGE:
    org-server parse [OPTIONS] <FILE>

ARGUMENTS:
    <FILE>    Path to the Org-mode file to parse

OPTIONS:
    -o, --output <OUTPUT_FILE>           Output file path (if not specified, outputs to stdout)
    -p, --pretty                         Pretty-print the JSON output
        --include-position               Include position information in the output
        --include-empty-sections         Include empty sections in the output
        --max-depth <MAX_DEPTH>          Maximum depth for nested sections [default: 10]
    -h, --help                           Print help information"
                );
            }
            Some("server") => {
                println!(
                    "Start the server mode for file monitoring and reminders

USAGE:
    org-server server [OPTIONS]

OPTIONS:
    -c, --config <CONFIG_FILE>    Configuration file path
    -p, --port <PORT>             Port to bind the server to [default: 3000]
        --host <HOST>             Host to bind the server to [default: 127.0.0.1]
    -h, --help                    Print help information"
                );
            }
            Some("help") => {
                println!(
                    "Print help information

USAGE:
    org-server help [SUBCOMMAND]

ARGUMENTS:
    [SUBCOMMAND]    Subcommand to show help for

Examples:
    org-server help
    org-server help parse
    org-server help server"
                );
            }
            _ => {
                println!(
                    "Org Server - Org-mode file parser and reminder server

USAGE:
    org-server [SUBCOMMAND]

SUBCOMMANDS:
    parse     Parse an Org-mode file and output JSON
    server    Start the server mode for file monitoring and reminders
    help      Print this message or the help of the given subcommand(s)

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information

Examples:
  org-server parse example.org --pretty
  org-server parse example.org --output output.json --include-position
  org-server server --port 8080 --host 0.0.0.0
  org-server server --config /path/to/config.toml"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_help_generation() {
        // Since we have manual help handling, we test the print_help function directly
        // This test ensures the help functionality works without relying on clap's help
        Cli::print_help(None);
        Cli::print_help(Some("parse"));
        Cli::print_help(Some("server"));
        Cli::print_help(Some("help"));
        // If we reach here without panicking, the help system works
    }

    #[test]
    fn test_help_subcommand_parsing() {
        let args = vec!["org-server", "help"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command.unwrap() {
            Commands::Help { subcommand } => {
                assert_eq!(subcommand, None);
            }
            _ => panic!("Expected Help command"),
        }
    }

    #[test]
    fn test_help_subcommand_with_arg() {
        let args = vec!["org-server", "help", "parse"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command.unwrap() {
            Commands::Help { subcommand } => {
                assert_eq!(subcommand, Some("parse".to_string()));
            }
            _ => panic!("Expected Help command"),
        }
    }
}
