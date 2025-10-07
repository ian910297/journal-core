# Stage 1: Build the application
FROM rust:1.73 as builder

# Create a new empty shell project
WORKDIR /usr/src/app
RUN USER=root cargo new --bin .

# Copy over the manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# Build only the dependencies to cache them
RUN cargo build --release
RUN rm src/*.rs

# Copy the source code and build the final binary
COPY ./src ./src
RUN rm ./target/release/deps/journal_core*
RUN cargo build --release

# Stage 2: Create the final, minimal image
FROM debian:buster-slim

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/app/target/release/journal-core .

# Set the command to run the application
CMD ["./journal-core"]
