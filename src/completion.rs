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
            } else if parts.len() == 2 {
                let command = parts[0];
                if let Some(_subcommands) = self.command_docs.get_subcommands(command) {
                    (start, self.complete_subcommands(command, last_part))
                } else {
                    (
                        start,
                        self.complete_args(command, parts.len() - 1, last_part),
                    )
                }
            } else {
                let command = parts[0];
                // Check if second part is a subcommand
                let potential_subcmd = parts[1];
                if let Some(_subcommands) = self.command_docs.get_subcommands(command) {
                    if _subcommands.contains_key(potential_subcmd) {
                        // We have a subcommand, complete its args
                        (
                            start,
                            self.complete_args(
                                &format!("{} {}", command, potential_subcmd),
                                parts.len() - 2,
                                last_part,
                            ),
                        )
                    } else {
                        // No subcommand, complete main command args
                        (
                            start,
                            self.complete_args(command, parts.len() - 1, last_part),
                        )
                    }
                } else {
                    // No subcommands, complete main command args
                    (
                        start,
                        self.complete_args(command, parts.len() - 1, last_part),
                    )
                }
            }
        };

        Ok((start, completions))
    }

    fn complete_commands(&self, prefix: &str) -> Vec<Pair> {
        let prefix = prefix.to_uppercase();
        let mut commands: Vec<_> = self
            .command_docs
            .all_commands()
            .iter()
            .filter(|cmd| cmd.starts_with(&prefix))
            .map(|cmd| Pair {
                display: cmd.to_string(),
                replacement: cmd.to_string(),
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
                        replacement: subcmd.to_string(),
                    })
                    .collect();

                // Sort subcommands alphabetically
                subcmds.sort_by(|a, b| a.display.cmp(&b.display));

                subcmds
            })
            .unwrap_or_default()
    }

    fn complete_args(&self, command: &str, _arg_index: usize, prefix: &str) -> Vec<Pair> {
        // This is a placeholder for argument completion
        // In a real implementation, we would parse the command's argument info
        // and provide relevant completions based on the argument type and position
        let mut completions: Vec<Pair> = Vec::new();

        // Add common Redis argument completions
        let common_args = vec!["EX", "PX", "EXAT", "PXAT", "KEEPTTL", "NX", "XX", "GET"];

        for arg in common_args {
            if arg.starts_with(prefix.to_uppercase().as_str()) {
                completions.push(Pair {
                    display: arg.to_string(),
                    replacement: arg.to_string(),
                });
            }
        }

        // Add key name completions for commands that operate on keys
        if self.is_key_operation(command) {
            let key_completions = self.complete_key_names(prefix);
            completions.extend(key_completions);
        }

        // Sort arguments alphabetically
        completions.sort_by(|a, b| a.display.cmp(&b.display));

        completions
    }

    fn complete_key_names(&self, prefix: &str) -> Vec<Pair> {
        // This is a placeholder for key name completion
        // In a real implementation, we would use SCAN to get matching keys
        let mut candidates = Vec::new();

        // For demonstration purposes, we'll just return some dummy keys
        let dummy_keys = vec!["key1", "key2", "user:1000", "user:1001", "product:100", "product:101"];

        for key in dummy_keys {
            if key.starts_with(prefix) {
                candidates.push(Pair {
                    display: key.to_string(),
                    replacement: key.to_string(),
                });
            }
        }

        candidates
    }

    fn is_key_operation(&self, command: &str) -> bool {
        // Check if the command operates on keys
        let key_commands = vec!["GET", "SET", "DEL", "EXISTS", "INCR", "DECR", "EXPIRE", "TTL", "LPUSH", "RPUSH", "LPOP", "RPOP", "HGET", "HSET", "HDEL", "HMGET", "HMSET"];

        key_commands.contains(&command.to_uppercase().as_str())
    }
}
