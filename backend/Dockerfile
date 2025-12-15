# Dockerfile for structured_eda backend

# Stage 1: Build the application and sqlx-cli
# Using a bookworm-based rust image to match the final image and avoid glibc issues.
FROM rust:1.92.0-bookworm as builder

# Install dependencies needed for building
RUN apt-get update && apt-get install -y pkg-config openssl libssl-dev

# Create a new empty cargo project to cache dependencies
WORKDIR /usr/src/structured_eda
COPY . .

# Install sqlx-cli
RUN cargo install sqlx-cli --version=0.7.4

# Build the backend
RUN cargo build --release

# Stage 2: Final lean image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y openssl ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy the application binary from the builder stage
COPY --from=builder /usr/src/structured_eda/target/release/eda-platform-backend /usr/local/bin/eda-platform-backend

# Copy the sqlx-cli binary from the builder stage
COPY --from=builder /usr/local/cargo/bin/sqlx /usr/local/bin/sqlx

# Copy migrations. The context is ./backend, so the path is relative to that
COPY ./migrations /app/migrations

# Set the working directory
WORKDIR /app