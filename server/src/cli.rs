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
}

impl Cli {
    pub fn get_command(&self) -> Commands {
        self.command.clone().unwrap_or(Commands::Server {
            config: None,
            port: 3000,
            host: "127.0.0.1".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_command_is_server() {
        let args = vec!["org-server"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.get_command() {
            Commands::Server { port, host, .. } => {
                assert_eq!(port, 3000);
                assert_eq!(host, "127.0.0.1");
            }
            _ => panic!("Expected Server command as default"),
        }
    }

    #[test]
    fn test_parse_subcommand() {
        let args = vec!["org-server", "parse", "test.org"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command.unwrap() {
            Commands::Parse { file, pretty, .. } => {
                assert_eq!(file, PathBuf::from("test.org"));
                assert!(!pretty);
            }
            _ => panic!("Expected Parse command"),
        }
    }

    #[test]
    fn test_server_subcommand_with_port() {
        let args = vec!["org-server", "server", "--port", "8080"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command.unwrap() {
            Commands::Server { port, .. } => {
                assert_eq!(port, 8080);
            }
            _ => panic!("Expected Server command"),
        }
    }
}
