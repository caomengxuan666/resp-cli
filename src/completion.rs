use rustyline::completion::Pair;
use rustyline::error::ReadlineError;

use crate::command_docs::CommandDocs;

pub struct CommandCompleter {
    command_docs: CommandDocs,
}

impl CommandCompleter {
    pub fn new(command_docs: CommandDocs) -> Self {
        Self { command_docs }
    }
    
    pub fn complete(&self, line: &str, pos: usize) -> Result<(usize, Vec<Pair>), ReadlineError> {
        let line = &line[..pos];
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        let (start, completions) = if parts.is_empty() {
            (0, self.complete_commands(""))
        } else {
            let last_part = parts.last().unwrap();
            let start = if let Some(space_pos) = line.rfind(' ') {
                space_pos + 1
            } else {
                0
            };
            
            if parts.len() == 1 {
                (start, self.complete_commands(last_part))
            } else {
                let command = parts[0];
                (start, self.complete_subcommands(command, last_part))
            }
        };
        
        Ok((start, completions))
    }
    
    fn complete_commands(&self, prefix: &str) -> Vec<Pair> {
        let prefix = prefix.to_uppercase();
        let mut commands: Vec<_> = self.command_docs
            .all_commands()
            .iter()
            .filter(|cmd| cmd.starts_with(&prefix))
            .map(|cmd| Pair { 
                display: cmd.to_string(), 
                replacement: cmd.to_string() 
            })
            .collect();
        
        // Sort commands alphabetically
        commands.sort_by(|a, b| a.display.cmp(&b.display));
        
        commands
    }
    
    fn complete_subcommands(&self, command: &str, prefix: &str) -> Vec<Pair> {
        let prefix = prefix.to_uppercase();
        self.command_docs
            .get_subcommands(command)
            .map(|subcommands| {
                let mut subcmds: Vec<_> = subcommands
                    .keys()
                    .filter(|subcmd| subcmd.starts_with(&prefix))
                    .map(|subcmd| Pair { 
                        display: subcmd.to_string(), 
                        replacement: subcmd.to_string() 
                    })
                    .collect();
                
                // Sort subcommands alphabetically
                subcmds.sort_by(|a, b| a.display.cmp(&b.display));
                
                subcmds
            })
            .unwrap_or_default()
    }
}
