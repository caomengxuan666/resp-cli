//! Config module
//!
//! Handles configuration parsing and loading.

use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(short = 'H', long, default_value = "localhost")]
    pub host: String,

    #[arg(short = 'P', long, default_value = "6379")]
    pub port: String,

    #[arg(short = 'a', long)]
    pub password: Option<String>,

    #[arg(long)]
    pub unix: Option<String>,

    #[arg(long)]
    pub tls: bool, // Enable TLS

    #[arg(long)]
    pub tls_ca_cert: Option<String>, // TLS CA certificate file

    #[arg(long)]
    pub tls_client_cert: Option<String>, // TLS client certificate file

    #[arg(long)]
    pub tls_client_key: Option<String>, // TLS client key file

    #[arg(short = 'n', long, default_value = "0")]
    pub db: i64, // Database number

    #[arg(short = 'r', long)]
    pub repeat: Option<u64>, // Repeat command N times

    #[arg(short = 'i', long, default_value = "0")]
    pub interval: f64, // Interval between repetitions (in seconds)

    #[arg(long)]
    pub raw: bool, // Raw output mode

    #[arg(short = 'x')]
    pub from_stdin: bool, // Read last argument from stdin

    #[arg(long)]
    pub scan: bool, // Scan mode

    #[arg(long)]
    pub client_name: Option<String>, // Client name

    #[arg(long)]
    pub cluster: bool, // Enable cluster mode

    #[arg(long, action = clap::ArgAction::Append)]
    pub cluster_nodes: Vec<String>, // Cluster nodes (host:port)

    #[arg(trailing_var_arg = true)]
    pub command: Vec<String>, // Command and arguments
}

/// Configuration struct for resp-cli
#[derive(Debug, Default)]
pub struct Config {
    // Connection settings
    pub host: String,
    pub port: String,
    pub password: Option<String>,
    pub unix: Option<String>,
    pub tls: bool,
    pub tls_ca_cert: Option<String>,
    pub tls_client_cert: Option<String>,
    pub tls_client_key: Option<String>,
    pub db: i64,

    // Command execution settings
    pub repeat: Option<u64>,
    pub interval: f64,
    pub raw: bool,
    pub from_stdin: bool,
    pub scan: bool,
    pub client_name: Option<String>,

    // UI settings
    pub syntax_highlighting: bool,
    pub color_theme: String,
    pub history_size: usize,
    pub completion_enabled: bool,
    pub key_completion_enabled: bool,

    // Cluster settings
    pub cluster: bool,
    pub cluster_nodes: Vec<String>,
}

/// Read .respclirc file from home directory
pub fn read_respclirc() -> Config {
    let mut config = Config {
        host: "localhost".to_string(),
        port: "6379".to_string(),
        db: 0,
        interval: 0.0,
        syntax_highlighting: true,
        color_theme: "default".to_string(),
        history_size: 1000,
        completion_enabled: true,
        key_completion_enabled: true,
        cluster: false,
        cluster_nodes: Vec::new(),
        ..Default::default()
    };

    // Get home directory
    if let Some(home) = dirs::home_dir() {
        let config_path = home.join(".respclirc");
        if config_path.exists()
            && let Ok(content) = std::fs::read_to_string(config_path)
        {
            for line in content.lines() {
                let line = line.trim();
                if !line.is_empty()
                    && !line.starts_with('#')
                    && let Some((key, value)) = line.split_once(' ')
                {
                    match key {
                        "host" => config.host = value.to_string(),
                        "port" => config.port = value.to_string(),
                        "password" => config.password = Some(value.to_string()),
                        "unix" => config.unix = Some(value.to_string()),
                        "tls" => config.tls = value.parse().unwrap_or(false),
                        "tls-ca-cert" => config.tls_ca_cert = Some(value.to_string()),
                        "tls-client-cert" => {
                            config.tls_client_cert = Some(value.to_string())
                        }
                        "tls-client-key" => config.tls_client_key = Some(value.to_string()),
                        "db" => config.db = value.parse().unwrap_or(0),
                        "repeat" => config.repeat = value.parse().ok(),
                        "interval" => config.interval = value.parse().unwrap_or(0.0),
                        "raw" => config.raw = value.parse().unwrap_or(false),
                        "from-stdin" => config.from_stdin = value.parse().unwrap_or(false),
                        "scan" => config.scan = value.parse().unwrap_or(false),
                        "client-name" => config.client_name = Some(value.to_string()),
                        "syntax-highlighting" => {
                            config.syntax_highlighting = value.parse().unwrap_or(true)
                        }
                        "color-theme" => config.color_theme = value.to_string(),
                        "history-size" => {
                            config.history_size = value.parse().unwrap_or(1000)
                        }
                        "completion-enabled" => {
                            config.completion_enabled = value.parse().unwrap_or(true)
                        }
                        "key-completion-enabled" => {
                            config.key_completion_enabled = value.parse().unwrap_or(true)
                        }
                        "cluster" => config.cluster = value.parse().unwrap_or(false),
                        "cluster-nodes" => config.cluster_nodes.push(value.to_string()),
                        _ => {} // Ignore unknown keys
                    }
                }
            }
        }
    }

    config
}
