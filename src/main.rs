//! resp-cli main
//! 
//! Main entry point for the resp-cli Redis client.

use clap::Parser;
use colored::Colorize;
use std::io::Read;
use rustyline::error::ReadlineError;
use rustyline::{Config, Editor};

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read config from .respclirc
    let config = read_respclirc();
    
    // Parse command line arguments
    let args = Args::parse();

    // Use config values as defaults if command line arguments are not specified
    let host = if !args.host.is_empty() && args.host != "localhost" {
        &args.host
    } else if let Some(h) = config.get("host") {
        h
    } else {
        "localhost"
    };

    let port = if !args.port.is_empty() && args.port != "6379" {
        &args.port
    } else if let Some(p) = config.get("port") {
        p
    } else {
        "6379"
    };

    let password = if args.password.is_some() {
        args.password.as_deref()
    } else if let Some(p) = config.get("password") {
        Some(p.as_str())
    } else {
        None
    };

    let unix = if args.unix.is_some() {
        args.unix.as_deref()
    } else if let Some(u) = config.get("unix") {
        Some(u.as_str())
    } else {
        None
    };

    let tls = args.tls || config.contains_key("tls");

    let tls_ca_cert = if args.tls_ca_cert.is_some() {
        args.tls_ca_cert.as_deref()
    } else if let Some(c) = config.get("tls-ca-cert") {
        Some(c.as_str())
    } else {
        None
    };

    let tls_client_cert = if args.tls_client_cert.is_some() {
        args.tls_client_cert.as_deref()
    } else if let Some(c) = config.get("tls-client-cert") {
        Some(c.as_str())
    } else {
        None
    };

    let tls_client_key = if args.tls_client_key.is_some() {
        args.tls_client_key.as_deref()
    } else if let Some(k) = config.get("tls-client-key") {
        Some(k.as_str())
    } else {
        None
    };

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
    if args.db != 0 {
        let result: redis::RedisResult<()> = redis::cmd("SELECT").arg(args.db).query(&mut conn);
        if let Err(e) = result {
            println!("{}", format!("Warning: Failed to select database {}: {}", args.db, e).yellow());
        }
    }

    // Set client name if specified
    if let Some(client_name) = &args.client_name {
        let result: redis::RedisResult<()> = redis::cmd("CLIENT").arg("SETNAME").arg(client_name).query(&mut conn);
        if let Err(e) = result {
            println!("{}", format!("Warning: Failed to set client name: {}", e).yellow());
        }
    }

    // Check if there are command arguments
    let cmd_args: Vec<String> = std::env::args().skip(1).collect();
    let mut command_parts: Vec<String> = Vec::new();
    let mut i = 0;
    while i < cmd_args.len() {
        let arg = &cmd_args[i];
        if arg.starts_with('-') {
            // This is an option
            if arg == "-r" || arg == "--repeat" || arg == "-i" || arg == "--interval" {
                // These options have values, skip the next argument
                i += 2;
            } else if arg == "-n" || arg == "--db" || arg == "-a" || arg == "--password" || 
                      arg == "-H" || arg == "--host" || arg == "-P" || arg == "--port" ||
                      arg == "--unix" || arg == "--tls" || arg == "--tls-ca-cert" ||
                      arg == "--tls-client-cert" || arg == "--tls-client-key" {
                // These options have values, skip the next argument
                i += 2;
            } else {
                // These options don't have values
                i += 1;
            }
        } else {
            // This is a command or argument
            command_parts.push(arg.clone());
            i += 1;
        }
    }

    // Handle scan mode
    if args.scan {
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
        if args.from_stdin {
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
        let repeat_count = args.repeat.unwrap_or(1);
        for i in 0..repeat_count {
            let result = redis::cmd(&final_command_parts[0])
                .arg(&final_command_parts[1..])
                .query(&mut conn);

            match result {
                Ok(value) => {
                    if args.raw {
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
            if args.interval > 0.0 && i < repeat_count - 1 {
                std::thread::sleep(std::time::Duration::from_secs_f64(args.interval));
            }
        }
        return Ok(());
    }

    // Fetch command documentation silently
    let command_docs = CommandDocs::fetch(&mut conn)?;

    let completer = CommandCompleter::new(command_docs);
    let config = Config::builder()
        .history_ignore_space(true)
        .auto_add_history(true)
        .build();

    let h = MyHelper { completer };

    let mut rl = Editor::with_config(config)?;
    rl.set_helper(Some(h));

    // Load history if it exists
    let _ = rl.load_history("resp-cli-history.txt");

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

    // Command timeout (in milliseconds)
    let mut timeout: Option<u64> = None;

    // Print welcome message
    print_welcome();

    // Track current database number
    let mut current_db = args.db;

    loop {
        let connection_info = format!("{}:{}", args.host, args.port);
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
    let _ = rl.save_history("resp-cli-history.txt");

    println!("{}", "Goodbye!".cyan());
    Ok(())
}
