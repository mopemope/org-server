use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Org Server - Org-mode file parser and reminder server
#[derive(Parser, Debug)]
#[command(name = "org-server")]
#[command(about = "A server for parsing Org-mode files and managing reminders")]
#[command(version)]
#[command(subcommand_required = false)]
#[command(arg_required_else_help = false)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Parse an Org-mode file and output JSON
    Parse {
        /// Path to the Org-mode file to parse
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Output file path (if not specified, outputs to stdout)
        #[arg(short, long, value_name = "OUTPUT_FILE")]
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

    /// Start the server mode (default behavior)
    Server {
        /// Configuration file path
        #[arg(short, long, value_name = "CONFIG_FILE")]
        config: Option<PathBuf>,

        /// Port to bind the server to
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Host to bind the server to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },
}

impl Default for Cli {
    fn default() -> Self {
        Self {
            command: Some(Commands::Server {
                config: None,
                port: 3000,
                host: "127.0.0.1".to_string(),
            }),
        }
    }
}

impl Cli {
    /// Parse command line arguments
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Get the command, defaulting to Server mode if none specified
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
    use clap::CommandFactory;

    #[test]
    fn test_cli_parse_command() {
        // Test parse command with minimal arguments
        let args = vec!["org-server", "parse", "test.org"];
        let cli = Cli::try_parse_from(args).unwrap();
        
        match cli.command.unwrap() {
            Commands::Parse { file, output, pretty, .. } => {
                assert_eq!(file, PathBuf::from("test.org"));
                assert_eq!(output, None);
                assert!(!pretty);
            }
            _ => panic!("Expected Parse command"),
        }
    }

    #[test]
    fn test_cli_parse_command_with_options() {
        // Test parse command with all options
        let args = vec![
            "org-server", "parse", "test.org",
            "--output", "output.json",
            "--pretty",
            "--include-position",
            "--include-empty-sections",
            "--max-depth", "5"
        ];
        let cli = Cli::try_parse_from(args).unwrap();
        
        match cli.command.unwrap() {
            Commands::Parse { 
                file, 
                output, 
                pretty, 
                include_position,
                include_empty_sections,
                max_depth,
            } => {
                assert_eq!(file, PathBuf::from("test.org"));
                assert_eq!(output, Some(PathBuf::from("output.json")));
                assert!(pretty);
                assert!(include_position);
                assert!(include_empty_sections);
                assert_eq!(max_depth, 5);
            }
            _ => panic!("Expected Parse command"),
        }
    }

    #[test]
    fn test_cli_server_command() {
        // Test server command with default values
        let args = vec!["org-server", "server"];
        let cli = Cli::try_parse_from(args).unwrap();
        
        match cli.command.unwrap() {
            Commands::Server { config, port, host } => {
                assert_eq!(config, None);
                assert_eq!(port, 3000);
                assert_eq!(host, "127.0.0.1");
            }
            _ => panic!("Expected Server command"),
        }
    }

    #[test]
    fn test_cli_server_command_with_options() {
        // Test server command with custom options
        let args = vec![
            "org-server", "server",
            "--config", "config.toml",
            "--port", "8080",
            "--host", "0.0.0.0"
        ];
        let cli = Cli::try_parse_from(args).unwrap();
        
        match cli.command.unwrap() {
            Commands::Server { config, port, host } => {
                assert_eq!(config, Some(PathBuf::from("config.toml")));
                assert_eq!(port, 8080);
                assert_eq!(host, "0.0.0.0");
            }
            _ => panic!("Expected Server command"),
        }
    }

    #[test]
    fn test_cli_default_behavior() {
        // Test that no subcommand defaults to server mode
        let args = vec!["org-server"];
        let cli = Cli::try_parse_from(args).unwrap();
        
        // When no subcommand is provided, command should be None
        assert!(cli.command.is_none());
        
        // get_command() should return Server as default
        match cli.get_command() {
            Commands::Server { .. } => {}, // Expected
            _ => panic!("Expected default Server command"),
        }
    }

    #[test]
    fn test_cli_help_generation() {
        // Ensure help can be generated without panicking
        let cmd = Cli::command();
        let _ = cmd.try_get_matches_from(vec!["org-server", "--help"]);
    }
}
