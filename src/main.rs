//! resp-cli main
//! 
//! Main entry point for the resp-cli Redis client.

use clap::Parser;
use colored::Colorize;
use std::io::Read;
use rustyline::error::ReadlineError;
use rustyline::{Config, Editor};
use dirs::home_dir;
use std::path::PathBuf;

use resp_cli::Args;
use resp_cli::CommandDocs;
use resp_cli::CommandCompleter;
use resp_cli::MyHelper;
use resp_cli::connect;
use resp_cli::format_value;
use resp_cli::print_raw_value;
use resp_cli::process_command;
use resp_cli::read_respclirc;
use resp_cli::print_welcome;
use resp_cli::get_prompt;

/// Get the path to the history file
fn get_history_path() -> PathBuf {
    if let Some(home) = home_dir() {
        home.join(".resp-cli-history")
    } else {
        PathBuf::from("resp-cli-history.txt") // Fallback to current directory
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read config from .respclirc
    let mut config = read_respclirc();
    
    // Parse command line arguments
    let args = Args::parse();

    // Override config with command line arguments if provided
    if !args.host.is_empty() && args.host != "localhost" {
        config.host = args.host;
    }

    if !args.port.is_empty() && args.port != "6379" {
        config.port = args.port;
    }

    if args.password.is_some() {
        config.password = args.password;
    }

    if args.unix.is_some() {
        config.unix = args.unix;
    }

    if args.tls {
        config.tls = true;
    }

    if args.tls_ca_cert.is_some() {
        config.tls_ca_cert = args.tls_ca_cert;
    }

    if args.tls_client_cert.is_some() {
        config.tls_client_cert = args.tls_client_cert;
    }

    if args.tls_client_key.is_some() {
        config.tls_client_key = args.tls_client_key;
    }

    if args.db != 0 {
        config.db = args.db;
    }

    if args.repeat.is_some() {
        config.repeat = args.repeat;
    }

    if args.interval != 0.0 {
        config.interval = args.interval;
    }

    if args.raw {
        config.raw = true;
    }

    if args.from_stdin {
        config.from_stdin = true;
    }

    if args.scan {
        config.scan = true;
    }

    if args.client_name.is_some() {
        config.client_name = args.client_name;
    };

    // Extract connection parameters
    let host = config.host.as_str();
    let port = config.port.as_str();
    let password = config.password.as_deref();
    let unix = config.unix.as_deref();
    let tls = config.tls;
    let tls_ca_cert = config.tls_ca_cert.as_deref();
    let tls_client_cert = config.tls_client_cert.as_deref();
    let tls_client_key = config.tls_client_key.as_deref();

    let mut conn = connect(
        host,
        port,
        password,
        unix,
        tls,
        tls_ca_cert,
        tls_client_cert,
        tls_client_key,
    )?;

    // Select database if not default (0)
    if config.db != 0 {
        let result: redis::RedisResult<()> = redis::cmd("SELECT").arg(config.db).query(&mut conn);
        if let Err(e) = result {
            println!("{}", format!("Warning: Failed to select database {}: {}", config.db, e).yellow());
        }
    }

    // Set client name if specified
    if let Some(client_name) = &config.client_name {
        let result: redis::RedisResult<()> = redis::cmd("CLIENT").arg("SETNAME").arg(client_name).query(&mut conn);
        if let Err(e) = result {
            println!("{}", format!("Warning: Failed to set client name: {}", e).yellow());
        }
    }

    // Check if there are command arguments
    let command_parts = args.command;

    // Handle scan mode
    if config.scan {
        let mut cursor: u64 = 0;
        let pattern = if !command_parts.is_empty() {
            command_parts[0].clone()
        } else {
            "*".to_string()
        };

        loop {
            let result: redis::RedisResult<(u64, Vec<String>)> = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(pattern.clone())
                .arg("COUNT")
                .arg(100)
                .query(&mut conn);

            match result {
                Ok((new_cursor, keys)) => {
                    for key in keys {
                        println!("{}", key);
                    }
                    cursor = new_cursor;
                    if cursor == 0 {
                        break;
                    }
                }
                Err(e) => {
                    println!("{}", format!("Error: {}", e).red());
                    break;
                }
            }
        }
        return Ok(());
    }

    // If there are command arguments, execute them
    if !command_parts.is_empty() {
        // Handle from stdin
        let mut final_command_parts = command_parts;
        if config.from_stdin {
            // Read from stdin
            let mut stdin = std::io::stdin();
            let mut input = String::new();
            stdin.read_to_string(&mut input)?;
            let input = input.trim().to_string();
            if !input.is_empty() {
                // Replace the last argument with stdin input
                if !final_command_parts.is_empty() {
                    final_command_parts.pop();
                }
                final_command_parts.push(input);
            }
        }

        // Execute the command
        let repeat_count = config.repeat.unwrap_or(1);
        for i in 0..repeat_count {
            let result = redis::cmd(&final_command_parts[0])
                .arg(&final_command_parts[1..])
                .query(&mut conn);

            match result {
                Ok(value) => {
                    if config.raw {
                        // Raw output mode
                        print_raw_value(&value);
                    } else {
                        // Normal output mode
                        println!("{}", format_value(&value));
                    }
                }
                Err(e) => {
                    println!("{}", format!("Error: {}", e).red());
                }
            }

            // Wait for interval if specified
            if config.interval > 0.0 && i < repeat_count - 1 {
                std::thread::sleep(std::time::Duration::from_secs_f64(config.interval));
            }
        }
        return Ok(());
    }

    // Fetch command documentation silently
    let command_docs = CommandDocs::fetch(&mut conn)?;

    let completer = CommandCompleter::new(command_docs);
    let rl_config = Config::builder()
        .history_ignore_space(true)
        .auto_add_history(true)
        .build();

    let mut h = MyHelper { completer };

    // Set the connection for key completion
    h.set_connection(&mut conn as *mut _);

    let mut rl = Editor::with_config(rl_config)?;
    rl.set_helper(Some(h));

    // Load history from user's home directory
    let history_path = get_history_path();
    let _ = rl.load_history(history_path.to_str().unwrap());

    // Transaction state
    let mut in_transaction = false;
    let mut transaction_commands: Vec<(String, Vec<String>)> = Vec::new();

    // Pipeline state
    let mut in_pipeline = false;
    let mut pipeline_commands: Vec<(String, Vec<String>)> = Vec::new();

    // Pub/Sub state
    let mut in_subscription = false;

    // Monitor state
    let mut in_monitor = false;

    // Command aliases
    let mut aliases: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    // Add some default aliases
    aliases.insert("ls".to_string(), "KEYS *".to_string());
    aliases.insert("ll".to_string(), "KEYS *".to_string());
    aliases.insert("del".to_string(), "DEL".to_string());
    aliases.insert("setex".to_string(), "SETEX".to_string());
    aliases.insert("getset".to_string(), "GETSET".to_string());
    aliases.insert("incr".to_string(), "INCR".to_string());
    aliases.insert("decr".to_string(), "DECR".to_string());
    aliases.insert("expire".to_string(), "EXPIRE".to_string());
    aliases.insert("ttl".to_string(), "TTL".to_string());
    aliases.insert("ping".to_string(), "PING".to_string());
    aliases.insert("info".to_string(), "INFO".to_string());
    aliases.insert("keys".to_string(), "KEYS".to_string());
    aliases.insert("exists".to_string(), "EXISTS".to_string());
    aliases.insert("type".to_string(), "TYPE".to_string());
    aliases.insert("rename".to_string(), "RENAME".to_string());
    aliases.insert("dbsize".to_string(), "DBSIZE".to_string());
    aliases.insert("flushdb".to_string(), "FLUSHDB".to_string());
    aliases.insert("flushall".to_string(), "FLUSHALL".to_string());

    // Command timeout (in milliseconds)
    let mut timeout: Option<u64> = None;

    // Print welcome message
    print_welcome();

    // Track current database number
    let mut current_db = config.db;

    loop {
        let connection_info = format!("{}:{}", config.host, config.port);
        let db_info = if current_db != 0 {
            format!("[{}]", current_db)
        } else {
            String::new()
        };
        let prompt = get_prompt(&connection_info, &db_info, in_transaction, in_pipeline, in_subscription, in_monitor);
        let readline = rl.readline(&prompt);
        match readline {
            Ok(line) => {
                let line_str = line.trim();
                if line_str.is_empty() {
                    // For empty lines, just show a new prompt
                    println!();
                    continue;
                }

                if line_str == "exit" || line_str == "quit" {
                    // If in transaction, discard it first
                    if in_transaction {
                        let _ = redis::cmd("DISCARD").query::<()>(&mut conn);
                        println!("{}", "Transaction discarded".yellow());
                    }
                    // If in pipeline, clear it
                    if in_pipeline {
                        pipeline_commands.clear();
                        println!("{}", "Pipeline cleared".yellow());
                    }
                    // If in monitor, exit monitor mode
                    if in_monitor {
                        println!("{}", "Exited monitor mode".yellow());
                    }
                    break;
                }

                // Check if the line ends with a backslash for multi-line input
                if line_str.ends_with('\\') {
                    // Start collecting multi-line input
                    let mut multi_line = line_str.trim_end_matches('\\').to_string();

                    loop {
                        let readline = rl.readline("...> ");
                        match readline {
                            Ok(continued_line) => {
                                let continued_line_str = continued_line.trim();
                                if continued_line_str.ends_with('\\') {
                                    multi_line.push(' ');
                                    multi_line.push_str(continued_line_str.trim_end_matches('\\'));
                                } else {
                                    multi_line.push(' ');
                                    multi_line.push_str(continued_line_str);
                                    break;
                                }
                            }
                            Err(ReadlineError::Interrupted) => {
                                println!("{}", "^C".red());
                                break;
                            }
                            Err(ReadlineError::Eof) => {
                                println!("{}", "^D".red());
                                break;
                            }
                            Err(err) => {
                                println!("{}", format!("Error: {:?}", err).red());
                                break;
                            }
                        }
                    }

                    // Process the multi-line command
                    let parts: Vec<&str> = multi_line.split_whitespace().collect();
                    if !parts.is_empty() {
                        process_command(
                            &mut conn,
                            &parts,
                            &mut in_transaction,
                            &mut transaction_commands,
                            &mut in_pipeline,
                            &mut pipeline_commands,
                            &mut in_subscription,
                            &mut in_monitor,
                            &mut aliases,
                            &mut timeout,
                            &mut current_db,
                        );
                    }
                } else {
                    // Single line command
                    let parts: Vec<&str> = line_str.split_whitespace().collect();
                    if !parts.is_empty() {
                        process_command(
                            &mut conn,
                            &parts,
                            &mut in_transaction,
                            &mut transaction_commands,
                            &mut in_pipeline,
                            &mut pipeline_commands,
                            &mut in_subscription,
                            &mut in_monitor,
                            &mut aliases,
                            &mut timeout,
                            &mut current_db,
                        );
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                // If in transaction, discard it
                if in_transaction {
                    let _ = redis::cmd("DISCARD").query::<()>(&mut conn);
                    println!("{}", "Transaction discarded".yellow());
                    in_transaction = false;
                    transaction_commands.clear();
                } else if in_pipeline {
                    // If in pipeline, clear it
                    pipeline_commands.clear();
                    println!("{}", "Pipeline cleared".yellow());
                } else if in_subscription {
                    // If in subscription, exit subscription mode
                    println!("{}", "Exited subscription mode".yellow());
                } else if in_monitor {
                    // If in monitor, exit monitor mode
                    in_monitor = false;
                    println!("{}", "Exited monitor mode".yellow());
                } else {
                    println!("{}", "^C".red());
                    break;
                }
            }
            Err(ReadlineError::Eof) => {
                // If in transaction, discard it
                if in_transaction {
                    let _ = redis::cmd("DISCARD").query::<()>(&mut conn);
                    println!("{}", "Transaction discarded".yellow());
                }
                // If in pipeline, clear it
                if in_pipeline {
                    pipeline_commands.clear();
                    println!("{}", "Pipeline cleared".yellow());
                }
                // If in subscription, exit subscription mode
                if in_subscription {
                    println!("{}", "Exited subscription mode".yellow());
                }
                // If in monitor, exit monitor mode
                if in_monitor {
                    println!("{}", "Exited monitor mode".yellow());
                }
                println!("{}", "^D".red());
                break;
            }
            Err(err) => {
                println!("{}", format!("Error: {:?}", err).red());
                break;
            }
        }
    }

    // Save history
    let history_path = get_history_path();
    let _ = rl.save_history(history_path.to_str().unwrap());

    println!("{}", "Goodbye!".cyan());
    Ok(())
}
