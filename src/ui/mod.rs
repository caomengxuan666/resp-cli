//! UI module
//! 
//! Handles user interface related functionality.

use colored::Colorize;
use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};

use crate::CommandCompleter;

pub struct MyHelper {
    pub completer: CommandCompleter,
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

/// Print welcome message
pub fn print_welcome() {
    println!(
        "{}",
        "
Welcome to resp-cli!"
            .cyan()
    );
    println!("{}", "Type commands or 'exit' to quit.".cyan());
    println!("{}", "Use Tab for command completion.".cyan());
}

/// Get prompt based on connection info and state
pub fn get_prompt(connection_info: &str, db_info: &str, in_transaction: bool, in_pipeline: bool, in_subscription: bool, in_monitor: bool) -> String {
    if in_transaction {
        format!("resp(multi)[{}{}]> ", connection_info, db_info)
    } else if in_pipeline {
        format!("resp(pipeline)[{}{}]> ", connection_info, db_info)
    } else if in_subscription {
        format!("resp(sub)[{}{}]> ", connection_info, db_info)
    } else if in_monitor {
        format!("resp(monitor)[{}{}]> ", connection_info, db_info)
    } else {
        format!("resp[{}{}]> ", connection_info, db_info)
    }
}
