use rustyline::completion::Pair;
use rustyline::error::ReadlineError;
use std::cell::RefCell;
use std::rc::Rc;

use crate::commands::command_docs::CommandDocs;

use redis::Connection;
use redis::cluster::ClusterConnection;

// Enum to handle both regular and cluster connections
pub enum RedisConnection {
    Regular(Connection),
    Cluster(ClusterConnection),
}

// Implement ConnectionLike trait for RedisConnection
impl redis::ConnectionLike for RedisConnection {
    fn req_packed_command(&mut self, cmd: &[u8]) -> redis::RedisResult<redis::Value> {
        match self {
            RedisConnection::Regular(conn) => conn.req_packed_command(cmd),
            RedisConnection::Cluster(conn) => conn.req_packed_command(cmd),
        }
    }

    fn req_packed_commands(
        &mut self,
        cmd: &[u8],
        offset: usize,
        count: usize,
    ) -> redis::RedisResult<Vec<redis::Value>> {
        match self {
            RedisConnection::Regular(conn) => conn.req_packed_commands(cmd, offset, count),
            RedisConnection::Cluster(conn) => conn.req_packed_commands(cmd, offset, count),
        }
    }

    fn get_db(&self) -> i64 {
        match self {
            RedisConnection::Regular(conn) => conn.get_db(),
            RedisConnection::Cluster(conn) => conn.get_db(),
        }
    }

    fn check_connection(&mut self) -> bool {
        match self {
            RedisConnection::Regular(conn) => conn.check_connection(),
            RedisConnection::Cluster(conn) => conn.check_connection(),
        }
    }

    fn is_open(&self) -> bool {
        match self {
            RedisConnection::Regular(conn) => conn.is_open(),
            RedisConnection::Cluster(conn) => conn.is_open(),
        }
    }
}

pub struct CommandCompleter {
    command_docs: CommandDocs,
    conn: Option<Rc<RefCell<RedisConnection>>>,
}

impl CommandCompleter {
    pub fn new(command_docs: CommandDocs) -> Self {
        Self {
            command_docs,
            conn: None,
        }
    }

    pub fn set_connection(&mut self, conn: Rc<RefCell<RedisConnection>>) {
        self.conn = Some(conn);
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
        let mut candidates = Vec::new();

        // Check if we have a connection to the Redis server
        if let Some(ref conn_rc) = self.conn {
            if let Ok(mut conn) = conn_rc.try_borrow_mut() {
                // Use SCAN to get keys matching the prefix
                let pattern = format!("{}*", prefix);
                let mut cursor = 0;

                loop {
                    let result = redis::cmd("SCAN")
                        .arg(cursor)
                        .arg("MATCH")
                        .arg(pattern.as_str())
                        .arg("COUNT")
                        .arg(100)
                        .query::<(u64, Vec<String>)>(&mut *conn);

                    match result {
                        Ok((new_cursor, keys)) => {
                            for key in keys {
                                candidates.push(Pair {
                                    display: key.clone(),
                                    replacement: key,
                                });
                            }

                            cursor = new_cursor;
                            if cursor == 0 {
                                break;
                            }
                        }
                        Err(_) => break, // If there's an error, just return what we have
                    }
                }
            }
        } else {
            // If no connection is available, return empty (no dummy keys)
            candidates = vec![];
        }

        candidates
    }

    fn is_key_operation(&self, command: &str) -> bool {
        // Check if the command operates on keys
        let key_commands = vec![
            "GET", "SET", "DEL", "EXISTS", "INCR", "DECR", "EXPIRE", "TTL", "LPUSH", "RPUSH",
            "LPOP", "RPOP", "HGET", "HSET", "HDEL", "HMGET", "HMSET",
        ];

        key_commands.contains(&command.to_uppercase().as_str())
    }
}
