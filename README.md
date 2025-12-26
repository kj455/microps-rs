# microps-rs

A TCP/IP protocol stack implementation in Rust

## Overview

This is a Rust reimplementation of [microps](https://github.com/pandax381/microps), a user-space TCP/IP protocol stack. The implementation is done incrementally, building up the network stack layer by layer.

## Prerequisites

### Using just (recommended)

This project uses [just](https://github.com/casey/just) as a command runner.

```bash
# macOS
brew install just

# Or via cargo
cargo install just
```

## Quick Start

### Running the application

```bash
# Run with info-level logging
just run

# Run with debug-level logging
just run-debug
```

You can also set the log level manually:

```bash
RUST_LOG=info cargo run
RUST_LOG=debug cargo run
```

### Building

```bash
# Build in release mode
just build

# Or using cargo directly
cargo build --release
```

### Testing

```bash
just test
```

## Development Setup

### Using Docker (Recommended)

#### 1. Build the Docker image

```bash
just docker-build
# or
docker compose build
```

#### 2. Start the container

```bash
just docker-run
# or
docker compose up -d
```

#### 3. Enter the container

```bash
just docker-exec
# or
docker compose exec microps-rs /bin/bash
```

#### 4. Build inside the container

```bash
# Inside the container
cargo build --release
```

#### 5. Create TAP device (inside container)

```bash
# Inside the container
just tap
```

#### 6. Run tests

```bash
# Inside the container
cargo test
```

#### 7. Stop the container

```bash
just docker-down
# or
docker compose down
```

### Local Development

#### Required packages (Ubuntu/Debian)

```bash
sudo apt-get install -y \
    build-essential \
    iproute2 \
    iputils-ping \
    libpcap-dev \
    net-tools \
    tcpdump
```

#### Create TAP device

```bash
just tap
```

## Project Structure

```
microps-rs/
├── src/             # Source code
│   ├── main.rs      # Entry point
│   └── net.rs       # Network module
├── examples/        # Example applications
├── docs/            # Documentation
├── Cargo.toml       # Project manifest
├── justfile         # Command runner recipes
└── Dockerfile       # Docker configuration
```

## Implementation Roadmap

This project is implemented incrementally. Each step corresponds to the git tags `book_stepN_skeleton/complete` from the original microps project.

For more details, see [docs/RUST_DESIGN.md](docs/RUST_DESIGN.md).

## Available Commands

```bash
just build        # Build in release mode
just test         # Run tests
just clean        # Clean build artifacts
just run          # Run with info logging
just run-debug    # Run with debug logging
just tap          # Create TAP device
just docker-build # Build Docker image
just docker-run   # Start Docker container
just docker-exec  # Enter Docker container
just docker-down  # Stop Docker container
```

## License

MIT

## References

- [microps](https://github.com/pandax381/microps) - Original C implementation
