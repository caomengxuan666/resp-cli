//! Command executor module
//!
//! Handles command execution and processing.

use crate::ConnParams;
use crate::completion::RedisConnection;
use colored::Colorize;
use redis::Value;

use crate::format_value;

/// Print value in raw mode (no color, no formatting)
pub fn print_raw_value(value: &Value) {
    match value {
        Value::Nil => println!("(nil)"),
        Value::Int(v) => println!("{}", v),
        Value::BulkString(v) => println!("{}", String::from_utf8_lossy(v)),
        Value::Array(values) => {
            for (i, value) in values.iter().enumerate() {
                print!("{}", i + 1);
                match value {
                    Value::BulkString(v) => println!(" > {}", String::from_utf8_lossy(v)),
                    _ => print_raw_value(value),
                }
            }
        }
        Value::SimpleString(v) => println!("{}", v),
        Value::Okay => println!("OK"),
        _ => println!("{:?}", value),
    }
}

/// Handle Redis error and print friendly error message
fn handle_error(e: &redis::RedisError, command: &str) {
    let error_msg = e.to_string();
    if error_msg.contains("NOAUTH") {
        println!("{}", "Error: Authentication required".red());
        println!(
            "{}",
            "Please connect with the correct password using -a option".yellow()
        );
    } else if error_msg.contains("ERR wrong number of arguments") {
        println!(
            "{}",
            format!("Error: Wrong number of arguments for command '{}'", command).red()
        );
        println!(
            "{}",
            "Please check the command syntax and try again".yellow()
        );
    } else if error_msg.contains("ERR unknown command") {
        println!("{}", format!("Error: Unknown command '{}'", command).red());
        println!("{}", "Please check the command name and try again".yellow());
    } else if error_msg.contains("CONNECTION REFUSED") {
        println!("{}", "Error: Connection refused".red());
        println!(
            "{}",
            "Please check if the Redis server is running and accessible".yellow()
        );
    } else if error_msg.contains("ERR no such key") {
        println!("{}", "Error: Key does not exist".red());
        println!("{}", "Please check the key name and try again".yellow());
    } else if error_msg.contains("ERR WRONGPASS") {
        println!("{}", "Error: Wrong password".red());
        println!(
            "{}",
            "Please provide the correct password using -a option".yellow()
        );
    } else if error_msg.contains("ERR DB index is out of range") {
        println!("{}", "Error: Database index is out of range".red());
        println!(
            "{}",
            "Please use a database index between 0 and 15".yellow()
        );
    } else {
        println!("{}", format!("Error: {}", e).red());
    }
}

/// Process a command
pub fn process_command(
    conn: &mut RedisConnection,
    parts: &[&str],
    in_transaction: &mut bool,
    transaction_commands: &mut Vec<(String, Vec<String>)>,
    in_pipeline: &mut bool,
    pipeline_commands: &mut Vec<(String, Vec<String>)>,
    in_subscription: &mut bool,
    in_monitor: &mut bool,
    aliases: &mut std::collections::HashMap<String, String>,
    aliases_modified: &mut bool,
    timeout: &mut Option<u64>,
    current_db: &mut i64,
    conn_params: &ConnParams,
) {
    // Check if it's a client-side command
    if parts[0].eq_ignore_ascii_case("clear") {
        // Clear the terminal screen
        print!("\x1b[2J\x1b[1;1H");
        return;
    }

    // Check if it's a HELP command
    if parts[0].eq_ignore_ascii_case("help") {
        println!("{}", "resp-cli client commands:".green().bold());
        println!("  {} - Clear the terminal screen", "CLEAR".cyan());
        println!("  {} - Start a transaction", "MULTI".cyan());
        println!("  {} - Execute transaction", "EXEC".cyan());
        println!("  {} - Discard transaction/pipeline", "DISCARD".cyan());
        println!("  {} - Start pipeline mode", "PIPELINE".cyan());
        println!("  {} - Enter monitor mode", "MONITOR".cyan());
        println!("  {} - Subscribe to channels", "SUBSCRIBE".cyan());
        println!("  {} - Select database (0-15)", "SELECT <db>".cyan());
        println!(
            "  {} - Set/show command timeout",
            "TIMEOUT [ms|CLEAR]".cyan()
        );
        println!("  {} - Execute commands from file", "SOURCE <file>".cyan());
        println!(
            "  {} - Manage command aliases",
            "ALIAS [name cmd|CLEAR|EXPORT|IMPORT]".cyan()
        );
        println!("  {} - Show this help message", "HELP".cyan());
        println!("  {} - Exit the client", "EXIT / QUIT".cyan());
        println!();
        println!(
            "{}",
            "Use Tab for command and subcommand completion.".dimmed()
        );
        println!(
            "{}",
            "Use backslash (\\) at end of line for multi-line input.".dimmed()
        );
        return;
    }

    // Check if it's an ALIAS command
    if parts[0].eq_ignore_ascii_case("ALIAS") {
        // Handle ALIAS command separately to avoid borrowing issues
        if parts.len() == 1 {
            // List all aliases
            if aliases.is_empty() {
                println!("{}", "No aliases defined".dimmed());
            } else {
                println!("{}", "Defined aliases:".green());
                for (alias, command) in aliases.iter() {
                    println!("  {} -> {}", alias, command);
                }
            }
        } else if parts.len() == 2 && parts[1] == "CLEAR" {
            // Clear all aliases
            aliases.clear();
            *aliases_modified = true;
            println!("{}", "All aliases cleared".green().bold());
        } else if parts.len() == 3 && parts[1] == "EXPORT" {
            // Export aliases to file
            let file_path = parts[2];
            match std::fs::write(file_path, serde_json::to_string_pretty(aliases).unwrap()) {
                Ok(_) => println!(
                    "{}",
                    format!("Aliases exported to '{}'", file_path)
                        .green()
                        .bold()
                ),
                Err(e) => println!("{}", format!("Error exporting aliases: {}", e).red()),
            }
        } else if parts.len() == 3 && parts[1] == "IMPORT" {
            // Import aliases from file
            let file_path = parts[2];
            match std::fs::read_to_string(file_path) {
                Ok(content) => {
                    match serde_json::from_str::<std::collections::HashMap<String, String>>(
                        &content,
                    ) {
                        Ok(imported_aliases) => {
                            aliases.extend(imported_aliases);
                            *aliases_modified = true;
                            println!(
                                "{}",
                                format!("Aliases imported from '{}'", file_path)
                                    .green()
                                    .bold()
                            );
                        }
                        Err(e) => println!("{}", format!("Error parsing alias file: {}", e).red()),
                    }
                }
                Err(e) => println!("{}", format!("Error reading alias file: {}", e).red()),
            }
        } else if parts.len() == 3 {
            // Set an alias
            let alias = parts[1].to_lowercase();
            let cmd = parts[2];
            aliases.insert(alias.clone(), cmd.to_string());
            *aliases_modified = true;
            println!(
                "{}",
                format!("Alias '{}' set to '{}'", alias, cmd).green().bold()
            );
        } else if parts.len() > 3 {
            // Set an alias with arguments
            let alias = parts[1].to_lowercase();
            let cmd = parts[2..].join(" ");
            aliases.insert(alias.clone(), cmd.clone());
            *aliases_modified = true;
            println!(
                "{}",
                format!("Alias '{}' set to '{}'", alias, cmd).green().bold()
            );
        } else {
            println!("{}", "Usage: ALIAS [alias] [command] or ALIAS CLEAR or ALIAS EXPORT <file> or ALIAS IMPORT <file>".red());
        }
        return;
    }

    // Process aliases for other commands
    let mut command_parts = parts.to_vec();
    let first_part = parts[0].to_lowercase();
    if let Some(alias) = aliases.get(&first_part) {
        let alias_parts: Vec<&str> = alias.split_whitespace().collect();
        if !alias_parts.is_empty() {
            command_parts[0] = alias_parts[0];
            // Add any additional arguments from the alias
            if alias_parts.len() > 1 {
                command_parts.splice(1..1, alias_parts[1..].iter().copied());
            }
        }
    }

    let command = command_parts[0].to_uppercase();

    match command.as_str() {
        "MULTI" => {
            if *in_pipeline {
                println!("{}", "Cannot start transaction in pipeline mode".red());
                return;
            }
            if *in_subscription {
                println!("{}", "Cannot start transaction in subscription mode".red());
                return;
            }
            if *in_monitor {
                println!("{}", "Cannot start transaction in monitor mode".red());
                return;
            }

            let result = redis::cmd("MULTI").query::<()>(conn);
            match result {
                Ok(_) => {
                    println!("{}", "OK".green().bold());
                    *in_transaction = true;
                    transaction_commands.clear();
                }
                Err(e) => {
                    println!("{}", format!("Error: {}", e).red());
                }
            }
        }
        "EXEC" => {
            if *in_transaction {
                // Execute the transaction
                let result = redis::cmd("EXEC").query(conn);
                match result {
                    Ok(value) => match value {
                        Value::Array(values) => {
                            for (i, value) in values.iter().enumerate() {
                                println!("{}> {}", i + 1, format_value(value));
                            }
                        }
                        Value::Nil => {
                            println!("{}", "(nil)".dimmed().italic());
                        }
                        _ => {
                            println!("{}", format_value(&value));
                        }
                    },
                    Err(e) => {
                        handle_error(&e, command_parts[0]);
                    }
                }
                *in_transaction = false;
                transaction_commands.clear();
            } else if *in_pipeline {
                // Execute the pipeline
                let mut pipe = redis::pipe();

                for (cmd_name, cmd_args) in pipeline_commands.iter() {
                    let cmd = pipe.cmd(cmd_name);
                    for arg in cmd_args {
                        cmd.arg(arg);
                    }
                }

                let result = pipe.query(conn);
                match result {
                    Ok(value) => match value {
                        Value::Array(values) => {
                            for (i, value) in values.iter().enumerate() {
                                println!("{}> {}", i + 1, format_value(value));
                            }
                        }
                        _ => {
                            println!("{}", format_value(&value));
                        }
                    },
                    Err(e) => {
                        handle_error(&e, command_parts[0]);
                    }
                }

                *in_pipeline = false;
                pipeline_commands.clear();
            } else {
                println!("{}", "Not in transaction or pipeline".red());
            }
        }
        "DISCARD" => {
            if *in_transaction {
                let result = redis::cmd("DISCARD").query::<()>(conn);
                match result {
                    Ok(_) => {
                        println!("{}", "OK".green().bold());
                        *in_transaction = false;
                        transaction_commands.clear();
                    }
                    Err(e) => {
                        handle_error(&e, command_parts[0]);
                    }
                }
            } else if *in_pipeline {
                // Clear pipeline
                pipeline_commands.clear();
                *in_pipeline = false;
                println!("{}", "Pipeline cleared".green().bold());
            } else {
                println!("{}", "Not in transaction or pipeline".red());
            }
        }
        "PIPELINE" => {
            if *in_transaction {
                println!("{}", "Cannot start pipeline in transaction mode".red());
                return;
            }
            if *in_subscription {
                println!("{}", "Cannot start pipeline in subscription mode".red());
                return;
            }
            if *in_monitor {
                println!("{}", "Cannot start pipeline in monitor mode".red());
                return;
            }

            *in_pipeline = true;
            pipeline_commands.clear();
            println!("{}", "Pipeline started".green().bold());
        }
        "MONITOR" => {
            if *in_transaction {
                println!("{}", "Cannot start monitor in transaction mode".red());
                return;
            }
            if *in_pipeline {
                println!("{}", "Cannot start monitor in pipeline mode".red());
                return;
            }
            if *in_subscription {
                println!("{}", "Cannot start monitor in subscription mode".red());
                return;
            }

            // Start monitor mode
            *in_monitor = true;

            // Create a new connection for monitoring (to avoid blocking the main connection)
            let mut monitor_conn = match crate::connect(
                &conn_params.host,
                &conn_params.port,
                conn_params.password.as_deref(),
                conn_params.unix.as_deref(),
                conn_params.tls,
                conn_params.tls_ca_cert.as_deref(),
                conn_params.tls_client_cert.as_deref(),
                conn_params.tls_client_key.as_deref(),
            ) {
                Ok(conn) => conn,
                Err(e) => {
                    println!("{}", format!("Error connecting for monitor: {}", e).red());
                    *in_monitor = false;
                    return;
                }
            };

            // Send MONITOR command using send_packed_command (does not read response)
            let result = monitor_conn.send_packed_command(&redis::cmd("MONITOR").get_packed_command());

            match result {
                Ok(_) => {
                    println!("{}", "OK".green().bold());
                    println!(
                        "{}",
                        "Entering monitor mode. Press Ctrl+C to exit.".yellow()
                    );
                }
                Err(e) => {
                    handle_error(&e, "MONITOR");
                    *in_monitor = false;
                    return;
                }
            }

            // Start receiving monitor messages using recv_response
            while let Ok(value) = monitor_conn.recv_response() {
                println!("{}", format_value(&value));
            }

            *in_monitor = false;
        }
        "SELECT" => {
            if command_parts.len() == 2 {
                if let Ok(db_num) = command_parts[1].parse::<i64>() {
                    let result = redis::cmd("SELECT").arg(db_num).query::<()>(conn);
                    match result {
                        Ok(_) => {
                            *current_db = db_num;
                            println!("{}", "OK".green().bold());
                        }
                        Err(e) => {
                            handle_error(&e, command_parts[0]);
                        }
                    }
                } else {
                    println!("{}", "Usage: SELECT <db_number>".red());
                }
            } else {
                println!("{}", "Usage: SELECT <db_number>".red());
            }
        }
        "TIMEOUT" => {
            if command_parts.len() == 1 {
                // Show current timeout
                if let Some(t) = *timeout {
                    println!("{}", format!("Current timeout: {} milliseconds", t).green());
                } else {
                    println!("{}", "No timeout set".dimmed());
                }
            } else if command_parts.len() == 2 {
                // Set timeout
                if command_parts[1] == "CLEAR" {
                    *timeout = None;
                    println!("{}", "Timeout cleared".green().bold());
                } else if let Ok(t) = command_parts[1].parse::<u64>() {
                    *timeout = Some(t);
                    println!(
                        "{}",
                        format!("Timeout set to {} milliseconds", t).green().bold()
                    );
                } else {
                    println!("{}", "Usage: TIMEOUT [milliseconds] or TIMEOUT CLEAR".red());
                }
            } else {
                println!("{}", "Usage: TIMEOUT [milliseconds] or TIMEOUT CLEAR".red());
            }
        }
        "SOURCE" => {
            if command_parts.len() == 2 {
                let file_path = command_parts[1];
                match std::fs::read_to_string(file_path) {
                    Ok(content) => {
                        let commands: Vec<&str> = content.split('\n').collect();
                        let mut success_count = 0;
                        let mut error_count = 0;

                        for (line_num, line) in commands.iter().enumerate() {
                            let trimmed_line = line.trim();
                            if trimmed_line.is_empty() || trimmed_line.starts_with('#') {
                                continue;
                            }

                            let parts: Vec<&str> = trimmed_line.split_whitespace().collect();
                            if !parts.is_empty() {
                                println!("{}", format!("Executing: {}", trimmed_line).blue());
                                let result = redis::cmd(parts[0]).arg(&parts[1..]).query(conn);

                                match result {
                                    Ok(value) => {
                                        println!("{}", format_value(&value));
                                        success_count += 1;
                                    }
                                    Err(e) => {
                                        println!(
                                            "{}",
                                            format!("Error at line {}:", line_num + 1).red()
                                        );
                                        handle_error(&e, parts[0]);
                                        error_count += 1;
                                    }
                                }
                            }
                        }

                        println!(
                            "{}",
                            format!(
                                "Execution completed: {} successful, {} failed",
                                success_count, error_count
                            )
                            .green()
                            .bold()
                        );
                    }
                    Err(e) => {
                        println!("{}", format!("Error reading file: {}", e).red());
                    }
                }
            } else {
                println!("{}", "Usage: SOURCE <file_path>".red());
            }
        }
        "CONFIG" => {
            if command_parts.len() == 1 {
                println!("{}", "Usage: CONFIG [GET|SET|RESETSTAT] [parameters]".red());
            } else if command_parts[1] == "GET" {
                if command_parts.len() == 2 {
                    println!("{}", "Usage: CONFIG GET <pattern>".red());
                } else {
                    let pattern = command_parts[2];
                    let result = redis::cmd("CONFIG").arg("GET").arg(pattern).query(conn);

                    match result {
                        Ok(value) => {
                            println!("{}", format_value(&value));
                        }
                        Err(e) => {
                            handle_error(&e, command_parts[0]);
                        }
                    }
                }
            } else if command_parts[1] == "SET" {
                if command_parts.len() < 4 {
                    println!("{}", "Usage: CONFIG SET <parameter> <value>".red());
                } else {
                    let parameter = command_parts[2];
                    let value = command_parts[3];
                    let result = redis::cmd("CONFIG")
                        .arg("SET")
                        .arg(parameter)
                        .arg(value)
                        .query(conn);

                    match result {
                        Ok(value) => {
                            println!("{}", format_value(&value));
                        }
                        Err(e) => {
                            handle_error(&e, command_parts[0]);
                        }
                    }
                }
            } else if command_parts[1] == "RESETSTAT" {
                let result = redis::cmd("CONFIG").arg("RESETSTAT").query(conn);

                match result {
                    Ok(value) => {
                        println!("{}", format_value(&value));
                    }
                    Err(e) => {
                        handle_error(&e, command_parts[0]);
                    }
                }
            } else {
                println!("{}", "Usage: CONFIG [GET|SET|RESETSTAT] [parameters]".red());
            }
        }
        "SUBSCRIBE" | "PSUBSCRIBE" => {
            if *in_transaction {
                println!("{}", "Cannot subscribe in transaction mode".red());
                return;
            }
            if *in_pipeline {
                println!("{}", "Cannot subscribe in pipeline mode".red());
                return;
            }
            if *in_monitor {
                println!("{}", "Cannot subscribe in monitor mode".red());
                return;
            }

            // Create a new regular connection for subscription (PubSub is not supported in cluster mode)
            let mut sub_conn = match crate::connect(
                &conn_params.host,
                &conn_params.port,
                conn_params.password.as_deref(),
                conn_params.unix.as_deref(),
                conn_params.tls,
                conn_params.tls_ca_cert.as_deref(),
                conn_params.tls_client_cert.as_deref(),
                conn_params.tls_client_key.as_deref(),
            ) {
                Ok(conn) => conn,
                Err(e) => {
                    println!(
                        "{}",
                        format!("Error connecting for subscription: {}", e).red()
                    );
                    return;
                }
            };

            // Use PubSub API for proper subscription handling
            let mut pubsub = sub_conn.as_pubsub();
            let is_pattern = command_parts[0].eq_ignore_ascii_case("PSUBSCRIBE");

            for channel in &command_parts[1..] {
                let result = if is_pattern {
                    pubsub.psubscribe(*channel)
                } else {
                    pubsub.subscribe(*channel)
                };
                match result {
                    Ok(_) => {
                        println!(
                            "{}",
                            format!("Subscribed to '{}'", channel).green().bold()
                        );
                    }
                    Err(e) => {
                        handle_error(&e, command_parts[0]);
                        return;
                    }
                }
            }

            *in_subscription = true;
            println!(
                "{}",
                "Listening for messages. Press Ctrl+C to exit.".yellow()
            );

            // Receive messages in a loop
            loop {
                match pubsub.get_message() {
                    Ok(msg) => {
                        let channel = msg.get_channel_name();
                        let payload: String = msg.get_payload().unwrap_or_default();
                        if is_pattern {
                            let pattern: String = msg.get_pattern().unwrap_or_default();
                            println!("{} [{}] {}", format!("[{}]", pattern).dimmed(), channel.cyan(), payload);
                        } else {
                            println!("{} {}", format!("[{}]", channel).cyan(), payload);
                        }
                    }
                    Err(e) => {
                        // Connection closed or error
                        let err_str = e.to_string();
                        if !err_str.contains("broken pipe") && !err_str.contains("connection closed") {
                            println!("{}", format!("Error: {}", e).red());
                        }
                        break;
                    }
                }
            }

            *in_subscription = false;
        }
        "UNSUBSCRIBE" | "PUNSUBSCRIBE" => {
            if !*in_subscription {
                println!("{}", "Not in subscription mode".red());
                return;
            }

            // Execute unsubscribe command
            let result = redis::cmd(command_parts[0])
                .arg(&command_parts[1..])
                .query(conn);

            match result {
                Ok(value) => {
                    println!("{}", format_value(&value));
                }
                Err(e) => {
                    println!("{}", format!("Error: {}", e).red());
                }
            }
        }
        "PUBLISH" => {
            if *in_transaction {
                // Add to transaction queue
                let cmd_name = command_parts[0].to_string();
                let cmd_args: Vec<String> =
                    command_parts[1..].iter().map(|s| s.to_string()).collect();
                transaction_commands.push((cmd_name, cmd_args));
                println!("{}", "QUEUED".green().bold());
            } else if *in_pipeline {
                // Add to pipeline queue
                let cmd_name = command_parts[0].to_string();
                let cmd_args: Vec<String> =
                    command_parts[1..].iter().map(|s| s.to_string()).collect();
                pipeline_commands.push((cmd_name, cmd_args));
                println!("{}", "QUEUED".green().bold());
            } else {
                // Execute immediately
                let result = redis::cmd(command_parts[0])
                    .arg(&command_parts[1..])
                    .query(conn);

                match result {
                    Ok(value) => {
                        println!("{}", format_value(&value));
                    }
                    Err(e) => {
                        handle_error(&e, command_parts[0]);
                    }
                }
            }
        }
        _ => {
            if *in_transaction {
                // Add to transaction queue
                let cmd_name = command_parts[0].to_string();
                let cmd_args: Vec<String> =
                    command_parts[1..].iter().map(|s| s.to_string()).collect();
                transaction_commands.push((cmd_name, cmd_args));
                println!("{}", "QUEUED".green().bold());
            } else if *in_pipeline {
                // Add to pipeline queue
                let cmd_name = command_parts[0].to_string();
                let cmd_args: Vec<String> =
                    command_parts[1..].iter().map(|s| s.to_string()).collect();
                pipeline_commands.push((cmd_name, cmd_args));
                println!("{}", "QUEUED".green().bold());
            } else if *in_subscription {
                // In subscription mode, only certain commands are allowed
                println!(
                    "{}",
                    "Only subscription-related commands are allowed in subscription mode".red()
                );
            } else if *in_monitor {
                // In monitor mode, only certain commands are allowed
                println!(
                    "{}",
                    "Only MONITOR command is allowed in monitor mode".red()
                );
            } else {
                // Execute immediately
                let result = redis::cmd(command_parts[0])
                    .arg(&command_parts[1..])
                    .query(conn);

                match result {
                    Ok(value) => {
                        println!("{}", format_value(&value));
                    }
                    Err(e) => {
                        handle_error(&e, command_parts[0]);
                    }
                }
            }
        }
    }
}
