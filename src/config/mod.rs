//! Config module
//! 
//! Handles configuration parsing and loading.

use clap::Parser;
use std::collections::HashMap;

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
}

/// Read .respclirc file from home directory
pub fn read_respclirc() -> HashMap<String, String> {
    let mut config = HashMap::new();
    
    // Get home directory
    if let Some(home) = std::env::var_os("HOME") {
        let config_path = std::path::Path::new(&home).join(".respclirc");
        if config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(config_path) {
                for line in content.lines() {
                    let line = line.trim();
                    if !line.is_empty() && !line.starts_with('#') {
                        if let Some((key, value)) = line.split_once(' ') {
                            config.insert(key.to_string(), value.to_string());
                        }
                    }
                }
            }
        }
    }
    
    config
}
