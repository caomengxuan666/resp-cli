use redis::RedisResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandInfo {
    name: String,
    arity: i32,
    flags: Vec<String>,
    first_key: i32,
    last_key: i32,
    key_step: i32,
    #[serde(default)]
    subcommands: Option<HashMap<String, CommandInfo>>,
    #[serde(default)]
    arguments: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArgumentInfo {
    name: String,
    #[serde(default)]
    type_: Option<String>,
    #[serde(default)]
    enum_: Option<Vec<String>>,
    #[serde(default)]
    optional: Option<bool>,
}

pub struct CommandDocs {
    commands: HashMap<String, CommandInfo>,
}

impl CommandDocs {
    pub fn fetch(conn: &mut redis::Connection) -> RedisResult<Self> {
        let mut commands = HashMap::new();

        // Try COMMAND DOCS first
        match redis::cmd("COMMAND").arg("DOCS").query(conn) {
            Ok(cmd_docs) => {
                if let redis::Value::Bulk(values) = &cmd_docs {
                    for value in values {
                        if let redis::Value::Data(data) = value {
                            if let Ok(cmd_info) = serde_json::from_slice(data) {
                                let cmd_info: CommandInfo = cmd_info;
                                commands.insert(cmd_info.name.to_uppercase(), cmd_info);
                            }
                        }
                    }
                }
            },
            Err(_) => {
                // COMMAND DOCS failed, fall back to COMMAND
                let cmd_list: redis::Value = redis::cmd("COMMAND").query(conn)?;

                if let redis::Value::Bulk(values) = &cmd_list {
                    for value in values {
                        if let redis::Value::Bulk(cmd_parts) = value {
                            if let Some(redis::Value::Data(cmd_name)) = cmd_parts.get(0) {
                                if let Ok(cmd_name_str) = String::from_utf8(cmd_name.to_vec()) {
                                    commands.insert(
                                        cmd_name_str.to_uppercase(),
                                        CommandInfo {
                                            name: cmd_name_str,
                                            arity: 0,
                                            flags: vec![],
                                            first_key: 0,
                                            last_key: 0,
                                            key_step: 0,
                                            subcommands: None,
                                            arguments: None,
                                        },
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        // If both commands failed or returned no data, try COMMAND as a last resort
        if commands.is_empty() {
            let cmd_list: redis::Value = redis::cmd("COMMAND").query(conn)?;

            if let redis::Value::Bulk(values) = &cmd_list {
                for value in values {
                    if let redis::Value::Bulk(cmd_parts) = value {
                        if let Some(redis::Value::Data(cmd_name)) = cmd_parts.get(0) {
                            if let Ok(cmd_name_str) = String::from_utf8(cmd_name.to_vec()) {
                                commands.insert(
                                    cmd_name_str.to_uppercase(),
                                    CommandInfo {
                                        name: cmd_name_str,
                                        arity: 0,
                                        flags: vec![],
                                        first_key: 0,
                                        last_key: 0,
                                        key_step: 0,
                                        subcommands: None,
                                        arguments: None,
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(Self { commands })
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn get_command(&self, name: &str) -> Option<&CommandInfo> {
        self.commands.get(&name.to_uppercase())
    }

    pub fn get_subcommands(&self, command: &str) -> Option<&HashMap<String, CommandInfo>> {
        self.get_command(command)
            .and_then(|cmd| cmd.subcommands.as_ref())
    }

    pub fn all_commands(&self) -> Vec<&String> {
        self.commands.keys().collect()
    }
}
