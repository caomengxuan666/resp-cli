# RESP-CLI Project Architecture

## Project Overview

RESP-CLI is a Redis-compatible command-line client that dynamically fetches command documentation from the server to provide real-time auto-completion. It supports Redis, Dragonfly, KeyDB, and other RESP-compatible servers.

## Project Structure

```
resp-cli/
├── Cargo.toml          # Project dependencies
├── src/
│   ├── main.rs         # Main entry point
│   ├── connection.rs   # Connection handling
│   ├── command_docs.rs # Command documentation fetching and parsing
│   ├── completion.rs   # Command auto-completion
│   └── formatter.rs    # Response formatting
└── AGENTS.MD           # Project architecture documentation
```

## Core Components

### 1. Connection Module (`connection.rs`)
- Handles establishing connections to Redis servers
- Supports TCP connections and Unix sockets
- Manages authentication with passwords

### 2. Command Docs Module (`command_docs.rs`)
- Fetches `COMMAND DOCS` from the server
- Parses the response into a structured format
- Provides access to command information and subcommands

### 3. Completion Module (`completion.rs`)
- Implements auto-completion based on fetched command documentation
- Supports completion of top-level commands and subcommands
- Provides case-insensitive matching

### 4. Formatter Module (`formatter.rs`)
- Formats Redis response values in a human-readable way
- Supports different types: strings, numbers, lists, hashes, etc.
- Adds color highlighting for better readability

### 5. Main Module (`main.rs`)
- Parses command-line arguments
- Establishes connection to the server
- Fetches command documentation
- Sets up the REPL environment with auto-completion
- Handles user input and command execution
- Manages history persistence

## Key Features

1. **Dynamic Command Completion**: Auto-completion based on real-time server command documentation
2. **Cross-Server Compatibility**: Works with any RESP-compatible server
3. **Interactive REPL**: User-friendly command-line interface with history
4. **Colorful Output**: Syntax highlighting for better readability
5. **Secure Connection**: Supports password authentication and Unix sockets

## Technical Stack

- **Rust**: Core programming language
- **rustyline**: Line editing and completion framework
- **redis-rs**: Redis client library
- **clap**: Command-line argument parsing
- **serde + serde_json**: JSON parsing for command documentation
- **colored**: Terminal color output

## Usage

```bash
# Connect to default Redis server (localhost:6379)
resp

# Connect to specific server
resp -h 127.0.0.1 -p 6379

# Connect with password
resp -h 127.0.0.1 -p 6379 -a mypassword

# Connect via Unix socket
resp --unix /path/to/redis.sock
```

## Auto-Completion

- **Tab Key**: Trigger auto-completion for commands and subcommands
- **Case Insensitive**: Matches commands regardless of case
- **Dynamic**: Based on server-provided command documentation
- **Subcommands**: Supports completion for nested subcommands

## Response Formatting

- **Strings**: Displayed in blue
- **Numbers**: Displayed in green
- **Nil**: Displayed in dimmed text
- **Lists/Sets**: Displayed with index numbers
- **Errors**: Displayed in red

## History

- **Persistent**: History is saved across sessions
- **Navigation**: Use up/down arrows to navigate history
- **Ignore Space**: Commands starting with space are not saved to history

## Exit Commands

- `exit` or `quit`: Exit the client
- `Ctrl+D`: EOF to exit
- `Ctrl+C`: Interrupt current command
