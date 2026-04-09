# RESP-CLI

A Redis-compatible command-line client with dynamic command completion and colorful output.

## Features

- **Dynamic Command Completion**: Auto-completion based on real-time server command documentation
- **Cross-Server Compatibility**: Works with any RESP-compatible server (Redis, Dragonfly, KeyDB)
- **Interactive REPL**: User-friendly command-line interface with history
- **Colorful Output**: Syntax highlighting for better readability
- **Secure Connection**: Supports password authentication and Unix sockets
- **Persistent History**: History is saved across sessions
- **Multi-line Input**: Support for multi-line commands
- **Friendly Error Messages**: Detailed error messages with suggestions

## Installation

### From Source

1. Clone the repository:

```bash
git clone https://github.com/yourusername/resp-cli.git
cd resp-cli
```

2. Build the project:

```bash
cargo build --release
```

3. Run the client:

```bash
./target/release/resp-cli
```

## Usage

### Basic Usage

```bash
# Connect to default Redis server (localhost:6379)
resp-cli

# Connect to specific server
resp-cli -H 127.0.0.1 -P 6379

# Connect with password
resp-cli -H 127.0.0.1 -P 6379 -a mypassword

# Connect via Unix socket
resp-cli --unix /path/to/redis.sock
```

### Command-Line Arguments

| Option | Short | Long | Description | Default |
|--------|-------|------|-------------|---------|
| Host | `-H` | `--host` | Redis server host | localhost |
| Port | `-P` | `--port` | Redis server port | 6379 |
| Password | `-a` | `--password` | Redis server password | None |
| Unix Socket | | `--unix` | Unix socket path | None |
| Help | `-h` | `--help` | Print help information | |
| Version | `-V` | `--version` | Print version information | |

### Interactive Mode

Once connected, you'll see the `resp> ` prompt. Here are some tips:

- **Auto-completion**: Press Tab to auto-complete commands, subcommands, and arguments
- **History**: Use up/down arrows to navigate command history
- **Multi-line input**: End a line with `\` to continue input on the next line
- **Exit**: Type `exit` or `quit`, or press Ctrl+D

### Examples

```bash
# Set a key
resp> SET mykey "Hello, World!"
OK

# Get a key
resp> GET mykey
"Hello, World!"

# Multi-line command
resp> SET mykey "This is a \
...> multi-line \
...> command"
OK

# List all keys
resp> KEYS *
[
    "mykey",
]
```

## Technical Stack

- **Rust**: Core programming language
- **rustyline**: Line editing and completion framework
- **redis-rs**: Redis client library
- **clap**: Command-line argument parsing
- **serde + serde_json**: JSON parsing for command documentation
- **colored**: Terminal color output

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

### Development Setup

1. Clone the repository
2. Run `cargo build` to build the project
3. Run `cargo run` to run the client in development mode
4. Run `cargo test` to run tests

### Code Style

This project follows the standard Rust code style. Please run `cargo fmt` before submitting a Pull Request.

## License

MIT
