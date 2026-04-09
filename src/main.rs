use clap::Parser;
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::{Config, Context, Editor, Helper};
use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;

mod connection;
mod command_docs;
mod completion;
mod formatter;

use connection::connect;
use command_docs::CommandDocs;
use completion::CommandCompleter;

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
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    println!("{}", "Connecting to Redis server...".blue());
    
    let mut conn = connect(
        &args.host,
        &args.port,
        args.password.as_deref(),
        args.unix.as_deref()
    )?;
    
    println!("{}", "Connected successfully!".green());
    println!("{}", "Fetching command documentation...".blue());
    
    let command_docs = CommandDocs::fetch(&mut conn)?;
    println!("{}", format!("Fetched {} commands from server", command_docs.len()).green());
    
    let completer = CommandCompleter::new(command_docs);
    let config = Config::builder()
        .history_ignore_space(true)
        .auto_add_history(true)
        .build();
    
    let h = MyHelper {
        completer,
    };
    
    let mut rl = Editor::with_config(config)?;
    rl.set_helper(Some(h));
    
    // Load history if it exists
    let _ = rl.load_history("resp-cli-history.txt");
    
    println!("{}", "
Welcome to resp-cli!".cyan());
    println!("{}", "Type commands or 'exit' to quit.".cyan());
    println!("{}", "Use Tab for command completion.".cyan());
    
    loop {
        let readline = rl.readline("resp> ");
        match readline {
            Ok(line) => {
                let line_str = line.trim();
                if line_str.is_empty() {
                    continue;
                }
                
                if line_str == "exit" || line_str == "quit" {
                    break;
                }
                
                let parts: Vec<&str> = line_str.split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }
                
                let result = redis::cmd(parts[0])
                    .arg(&parts[1..])
                    .query(&mut conn);
                
                match result {
                    Ok(value) => {
                        println!("{}", formatter::format_value(&value));
                    }
                    Err(e) => {
                        println!("{}", format!("Error: {}", e).red());
                    }
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
    
    // Save history
    let _ = rl.save_history("resp-cli-history.txt");
    
    println!("{}", "Goodbye!".cyan());
    Ok(())
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
impl Highlighter for MyHelper {}
impl Hinter for MyHelper {
    type Hint = String;
}
impl Validator for MyHelper {}
