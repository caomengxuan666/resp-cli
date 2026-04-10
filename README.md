# resp-cli

A Redis-compatible command-line client written in Rust, supporting Redis, Dragonfly, and KeyDB.

## Features

- **Real-time command documentation** fetching from the server
- **Tab auto-completion** based on server-provided command documentation
- **Manual completion triggering** support
- **TCP and Unix socket** connection support
- **Interactive REPL mode** with history
- **Colorful output** and error handling
- **Transaction support** (MULTI/EXEC/DISCARD)
- **Pipeline execution** for improved performance
- **Pub/Sub messaging** support
- **MONITOR mode** for server monitoring
- **Database selection** with SELECT command
- **Configuration file** support (.respclirc)
- **Client-side commands** like clear
- **Command aliases** for custom shortcuts
- **Scan mode** for key iteration
- **Raw output mode** for machine-readable output
- **Repeat command** functionality
- **Read from stdin** support
- **TLS encryption** support
- **Redis Cluster** support

## Installation

### From crates.io

```bash
cargo install resp-cli
```

### From Source

```bash
# Clone the repository
git clone https://github.com/yourusername/resp-cli.git
cd resp-cli

# Build the project
cargo build --release

# Run the client
./target/release/resp-cli
```

### From Pre-built Binaries

Pre-built binaries are available for Linux and Windows in the [releases](https://github.com/caomengxuan666/resp-cli/releases) section.

## Usage

### Basic Connection

```bash
# Connect to localhost:6379
resp-cli

# Connect to a specific host and port
resp-cli -H 127.0.0.1 -P 6379

# Connect using a Unix socket
resp-cli --unix /tmp/redis.sock

# Connect with password
resp-cli -a yourpassword

# Select database
resp-cli -n 1
```

### Command Line Options

- `-H, --host <HOST>`: Host to connect to (default: localhost)
- `-P, --port <PORT>`: Port to connect to (default: 6379)
- `-a, --password <PASSWORD>`: Password for authentication
- `--unix <PATH>`: Unix socket path
- `--tls`: Enable TLS encryption
- `--tls-ca-cert <PATH>`: TLS CA certificate file
- `--tls-client-cert <PATH>`: TLS client certificate file
- `--tls-client-key <PATH>`: TLS client key file
- `-n, --db <DB>`: Database number (default: 0)
- `-r, --repeat <COUNT>`: Repeat command N times
- `-i, --interval <SECONDS>`: Interval between repetitions (in seconds)
- `--raw`: Raw output mode
- `-x`: Read last argument from stdin
- `--scan`: Scan mode
- `--client-name <NAME>`: Client name

### Interactive Mode

When running resp-cli without command arguments, it enters interactive mode with a REPL (Read-Eval-Print Loop).

#### Key Features

- **Tab completion**: Press Tab to auto-complete commands, subcommands, and arguments
- **Command history**: Use up/down arrow keys to navigate through previous commands
- **Multi-line input**: End a line with backslash (\) to continue input on the next line
- **Client-side commands**: Use `clear` to clear the terminal screen
- **Command aliases**: Define custom aliases with the `ALIAS` command

#### Example Session

```
resp[localhost:6379]> set key value
OK
resp[localhost:6379]> get key
"value"
resp[localhost:6379]> keys *
1> "key"
resp[localhost:6379]> clear
# Screen cleared
resp[localhost:6379]> exit
Goodbye!
```

## Configuration

resp-cli supports a configuration file `.respclirc` in the home directory. The file can contain the following settings:

```
host 127.0.0.1
port 6379
password yourpassword
unix /tmp/redis.sock
tls
tls-ca-cert /path/to/ca.crt
tls-client-cert /path/to/client.crt
tls-client-key /path/to/client.key
```

## Commands

### Supported Redis Commands

resp-cli supports all Redis commands that the connected server provides. The client fetches command documentation from the server to provide auto-completion and better error messages.

### Client-Side Commands

- `clear`: Clear the terminal screen
- `ALIAS`: Manage command aliases
- `TIMEOUT`: Set command timeout
- `SOURCE`: Execute commands from a file

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT
