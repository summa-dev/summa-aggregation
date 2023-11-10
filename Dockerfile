# Build Stage
FROM rust:latest as builder

# Create a new empty shell project
RUN USER=root cargo new --bin mini-tree-server
WORKDIR /mini-tree-server

# Copy files
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./rust-toolchain ./rust-toolchain
COPY ./src ./src
COPY ./bin ./bin

# Build for release
RUN cargo build --release

# Base image for mini tree server
FROM ubuntu:latest

# Copy the binary from the builder stage
COPY --from=builder /mini-tree-server/target/release/mini-tree-server /usr/local/bin/

# Set the binary as the entrypoint of the container
ENTRYPOINT ["mini-tree-server"]
