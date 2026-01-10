# builder
FROM rust:latest AS builder

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./

RUN mkdir src && \
    echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

COPY . .

RUN touch src/main.rs && cargo build --release

# runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y libssl-dev libsqlite3-0 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/app/target/release/project /usr/local/bin/node

COPY Settings.toml .

COPY keys/ ./keys/

RUN mkdir -p /data

EXPOSE 6000

ENTRYPOINT ["node"]