use clap::Parser;
use colored::Colorize;
use rustyline::completion::{Completer, Pair};
use std::io::Read;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Config, Context, Editor, Helper};

mod command_docs;
mod completion;
mod connection;
mod formatter;

use command_docs::CommandDocs;
use completion::CommandCompleter;
use connection::connect;

// Print value in raw mode (no color, no formatting)
fn print_raw_value(value: &redis::Value) {
    match value {
        redis::Value::Nil => println!("(nil)"),
        redis::Value::Int(v) => println!("{}", v),
        redis::Value::Data(v) => println!("{}", String::from_utf8_lossy(v)),
        redis::Value::Bulk(values) => {
            for (i, value) in values.iter().enumerate() {
                print!("{}", i + 1);
                match value {
                    redis::Value::Data(v) => println!(" > {}", String::from_utf8_lossy(v)),
                    _ => print_raw_value(value),
                }
            }
        }
        redis::Value::Status(v) => println!("{}", v),
        redis::Value::Okay => println!("OK"),
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short = 'H', long, default_value = "localhost")]
    host: String,

    #[arg(short = 'P', long, default_value = "6379")]
    port: String,

    #[arg(short = 'a', long)]
    password: Option<String>,

    #[arg(long)]
    unix: Option<String>,

    #[arg(long)]
    tls: bool, // Enable TLS

    #[arg(long)]
    tls_ca_cert: Option<String>, // TLS CA certificate file

    #[arg(long)]
    tls_client_cert: Option<String>, // TLS client certificate file

    #[arg(long)]
    tls_client_key: Option<String>, // TLS client key file

    #[arg(short = 'n', long, default_value = "0")]
    db: i64, // Database number

    #[arg(short = 'r', long)]
    repeat: Option<u64>, // Repeat command N times

    #[arg(short = 'i', long, default_value = "0")]
    interval: f64, // Interval between repetitions (in seconds)

    #[arg(long)]
    raw: bool, // Raw output mode

    #[arg(short = 'x')]
    from_stdin: bool, // Read last argument from stdin

    #[arg(long)]
    scan: bool, // Scan mode

    #[arg(long)]
    client_name: Option<String>, // Client name
}

// Read .respclirc file from home directory
fn read_respclirc() -> std::collections::HashMap<String, String> {
    let mut config = std::collections::HashMap::new();
    
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read config from .respclirc
    let config = read_respclirc();
    
    // Parse command line arguments
    let args = Args::parse();

    println!("{}", "Connecting to Redis server...".blue());

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

    println!("{}", "Connected successfully!".green());

    // Select database if not default (0)
    if args.db != 0 {
        let result: redis::RedisResult<()> = redis::cmd("SELECT").arg(args.db).query(&mut conn);
        match result {
            Ok(_) => {
                println!("{}", format!("Selected database {}", args.db).green());
            }
            Err(e) => {
                println!("{}", format!("Warning: Failed to select database {}: {}", args.db, e).yellow());
            }
        }
    }

    // Set client name if specified
    if let Some(client_name) = &args.client_name {
        let result: redis::RedisResult<()> = redis::cmd("CLIENT").arg("SETNAME").arg(client_name).query(&mut conn);
        match result {
            Ok(_) => {
                println!("{}", format!("Set client name to '{}'", client_name).green());
            }
            Err(e) => {
                println!("{}", format!("Warning: Failed to set client name: {}", e).yellow());
            }
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
                        println!("{}", formatter::format_value(&value));
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

    println!("{}", "Fetching command documentation...".blue());

    let command_docs = CommandDocs::fetch(&mut conn)?;
    println!(
        "{}",
        format!("Fetched {} commands from server", command_docs.len()).green()
    );

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

    println!(
        "{}",
        "
Welcome to resp-cli!"
            .cyan()
    );
    println!("{}", "Type commands or 'exit' to quit.".cyan());
    println!("{}", "Use Tab for command completion.".cyan());

    // Track current database number
    let mut current_db = args.db;

    loop {
        let connection_info = format!("{}:{}", args.host, args.port);
        let db_info = if current_db != 0 {
            format!("[{}]", current_db)
        } else {
            String::new()
        };
        let prompt = if in_transaction {
            format!("resp(multi)[{}{}]> ", connection_info, db_info)
        } else if in_pipeline {
            format!("resp(pipeline)[{}{}]> ", connection_info, db_info)
        } else if in_subscription {
            format!("resp(sub)[{}{}]> ", connection_info, db_info)
        } else if in_monitor {
            format!("resp(monitor)[{}{}]> ", connection_info, db_info)
        } else {
            format!("resp[{}{}]> ", connection_info, db_info)
        };
        let readline = rl.readline(&prompt);
        match readline {
            Ok(line) => {
                let line_str = line.trim();
                if line_str.is_empty() {
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
                        in_monitor = false;
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
                    in_monitor = false;
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

fn process_command(
    conn: &mut redis::Connection,
    parts: &[&str],
    in_transaction: &mut bool,
    transaction_commands: &mut Vec<(String, Vec<String>)>,
    in_pipeline: &mut bool,
    pipeline_commands: &mut Vec<(String, Vec<String>)>,
    in_subscription: &mut bool,
    in_monitor: &mut bool,
    aliases: &mut std::collections::HashMap<String, String>,
    timeout: &mut Option<u64>,
    current_db: &mut i64,
) {
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
            println!("{}", "All aliases cleared".green().bold());
        } else if parts.len() == 3 {
            // Set an alias
            let alias = parts[1].to_lowercase();
            let cmd = parts[2];
            aliases.insert(alias.clone(), cmd.to_string());
            println!("{}", format!("Alias '{}' set to '{}'", alias, cmd).green().bold());
        } else if parts.len() > 3 {
            // Set an alias with arguments
            let alias = parts[1].to_lowercase();
            let cmd = parts[2..].join(" ");
            aliases.insert(alias.clone(), cmd.clone());
            println!("{}", format!("Alias '{}' set to '{}'", alias, cmd).green().bold());
        } else {
            println!("{}", "Usage: ALIAS [alias] [command] or ALIAS CLEAR".red());
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
                    Ok(value) => {
                        match value {
                            redis::Value::Bulk(values) => {
                                for (i, value) in values.iter().enumerate() {
                                    println!("{}> {}", i + 1, formatter::format_value(value));
                                }
                            }
                            redis::Value::Nil => {
                                println!("{}", "(nil)".dimmed().italic());
                            }
                            _ => {
                                println!("{}", formatter::format_value(&value));
                            }
                        }
                    }
                    Err(e) => {
                        println!("{}", format!("Error: {}", e).red());
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
                    Ok(value) => {
                        match value {
                            redis::Value::Bulk(values) => {
                                for (i, value) in values.iter().enumerate() {
                                    println!("{}> {}", i + 1, formatter::format_value(value));
                                }
                            }
                            _ => {
                                println!("{}", formatter::format_value(&value));
                            }
                        }
                    }
                    Err(e) => {
                        println!("{}", format!("Error: {}", e).red());
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
                        println!("{}", format!("Error: {}", e).red());
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
            let args = Args::parse();
            let mut monitor_conn = match connect(
                &args.host,
                &args.port,
                args.password.as_deref(),
                args.unix.as_deref(),
                args.tls,
                args.tls_ca_cert.as_deref(),
                args.tls_client_cert.as_deref(),
                args.tls_client_key.as_deref()
            ) {
                Ok(conn) => conn,
                Err(e) => {
                    println!("{}", format!("Error connecting for monitor: {}", e).red());
                    *in_monitor = false;
                    return;
                }
            };

            // Send MONITOR command
            let result = redis::cmd("MONITOR").query::<()>(&mut monitor_conn);

            match result {
                Ok(_) => {
                    println!("{}", "OK".green().bold());
                    println!("{}", "Entering monitor mode. Press Ctrl+C to exit.".yellow());
                }
                Err(e) => {
                    println!("{}", format!("Error: {}", e).red());
                    *in_monitor = false;
                    return;
                }
            }

            // Start receiving monitor messages
            loop {
                // Try to get the next message from the connection
                let result: redis::RedisResult<redis::Value> = redis::cmd("").query(&mut monitor_conn);
                match result {
                    Ok(value) => {
                        println!("{}", formatter::format_value(&value));
                    }
                    Err(_) => {
                        // Connection closed or error, exit monitor mode
                        break;
                    }
                }
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
                            println!("{}", format!("Error: {}", e).red());
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
                                        println!("{}", formatter::format_value(&value));
                                        success_count += 1;
                                    }
                                    Err(e) => {
                                        println!(
                                            "{}",
                                            format!("Error at line {}: {}", line_num + 1, e).red()
                                        );
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
                            println!("{}", formatter::format_value(&value));
                        }
                        Err(e) => {
                            println!("{}", format!("Error: {}", e).red());
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
                            println!("{}", formatter::format_value(&value));
                        }
                        Err(e) => {
                            println!("{}", format!("Error: {}", e).red());
                        }
                    }
                }
            } else if command_parts[1] == "RESETSTAT" {
                let result = redis::cmd("CONFIG").arg("RESETSTAT").query(conn);

                match result {
                    Ok(value) => {
                        println!("{}", formatter::format_value(&value));
                    }
                    Err(e) => {
                        println!("{}", format!("Error: {}", e).red());
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

            // Start subscription
            *in_subscription = true;

            // Create a new connection for subscription (to avoid blocking the main connection)
            let args = Args::parse();
            let mut sub_conn = match connect(
                &args.host,
                &args.port,
                args.password.as_deref(),
                args.unix.as_deref(),
                args.tls,
                args.tls_ca_cert.as_deref(),
                args.tls_client_cert.as_deref(),
                args.tls_client_key.as_deref(),
            ) {
                Ok(conn) => conn,
                Err(e) => {
                    println!(
                        "{}",
                        format!("Error connecting for subscription: {}", e).red()
                    );
                    *in_subscription = false;
                    return;
                }
            };

            // Send subscribe command
            let result = redis::cmd(command_parts[0])
                .arg(&command_parts[1..])
                .query(&mut sub_conn);

            match result {
                Ok(value) => {
                    println!("{}", formatter::format_value(&value));
                }
                Err(e) => {
                    println!("{}", format!("Error: {}", e).red());
                    *in_subscription = false;
                    return;
                }
            }
            println!(
                "{}",
                "Entering subscription mode. Press Ctrl+C to exit.".yellow()
            );

            // Start receiving messages
            loop {
                match redis::cmd("PING").query::<()>(&mut sub_conn) {
                    Ok(_) => {
                        // This should not happen in subscription mode
                        break;
                    }
                    Err(_) => {
                        // In subscription mode, we should receive messages instead of PING responses
                        match redis::cmd("").query(&mut sub_conn) {
                            Ok(value) => {
                                println!("{}", formatter::format_value(&value));
                            }
                            Err(e) => {
                                println!("{}", format!("Error receiving message: {}", e).red());
                                break;
                            }
                        }
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
                    println!("{}", formatter::format_value(&value));
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
                        println!("{}", formatter::format_value(&value));
                    }
                    Err(e) => {
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
                                format!(
                                    "Error: Wrong number of arguments for command '{}'",
                                    command_parts[0]
                                )
                                .red()
                            );
                            println!(
                                "{}",
                                "Please check the command syntax and try again".yellow()
                            );
                        } else if error_msg.contains("ERR unknown command") {
                            println!(
                                "{}",
                                format!("Error: Unknown command '{}'", command_parts[0]).red()
                            );
                            println!("{}", "Please check the command name and try again".yellow());
                        } else if error_msg.contains("CONNECTION REFUSED") {
                            println!("{}", "Error: Connection refused".red());
                            println!(
                                "{}",
                                "Please check if the Redis server is running and accessible"
                                    .yellow()
                            );
                        } else {
                            println!("{}", format!("Error: {}", e).red());
                        }
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
                        println!("{}", formatter::format_value(&value));
                    }
                    Err(e) => {
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
                                format!(
                                    "Error: Wrong number of arguments for command '{}'",
                                    command_parts[0]
                                )
                                .red()
                            );
                            println!(
                                "{}",
                                "Please check the command syntax and try again".yellow()
                            );
                        } else if error_msg.contains("ERR unknown command") {
                            println!(
                                "{}",
                                format!("Error: Unknown command '{}'", command_parts[0]).red()
                            );
                            println!("{}", "Please check the command name and try again".yellow());
                        } else if error_msg.contains("CONNECTION REFUSED") {
                            println!("{}", "Error: Connection refused".red());
                            println!(
                                "{}",
                                "Please check if the Redis server is running and accessible"
                                    .yellow()
                            );
                        } else {
                            println!("{}", format!("Error: {}", e).red());
                        }
                    }
                }
            }
        }
    }
}

struct MyHelper {
    completer: CommandCompleter,
}

impl Completer for MyHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        self.completer.complete(line, pos)
    }
}

impl Helper for MyHelper {}
impl Highlighter for MyHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> std::borrow::Cow<'l, str> {
        // Simple highlighting for now
        line.into()
    }

    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> std::borrow::Cow<'b, str> {
        if default {
            prompt.green().to_string().into()
        } else {
            prompt.to_string().into()
        }
    }
}
impl Hinter for MyHelper {
    type Hint = String;
}
impl Validator for MyHelper {}
