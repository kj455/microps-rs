tapdev := "tap0"
tapaddr := "192.0.2.1/24"

default: build

build:
    cargo build --release

test:
    cargo test

clean:
    cargo clean

run:
    RUST_LOG=info cargo run

run-debug:
    RUST_LOG=debug cargo run

tap:
    #!/usr/bin/env bash
    if ! ip addr show {{tapdev}} 2>/dev/null; then
        echo "Create '{{tapdev}}'"
        sudo ip tuntap add mode tap user ${USER:-root} name {{tapdev}}
        sudo sysctl -w net.ipv6.conf.{{tapdev}}.disable_ipv6=1 2>/dev/null || true
        sudo ip addr add {{tapaddr}} dev {{tapdev}}
        sudo ip link set {{tapdev}} up
        ip addr show {{tapdev}}
    fi

docker-build:
    docker compose build

docker-run:
    docker compose up -d

docker-exec:
    docker compose exec microps-rs /bin/bash

docker-down:
    docker compose down
