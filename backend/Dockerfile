# Build stage
FROM rust:1.92 AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install required system dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    git \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1001 app

# Create data directory
RUN mkdir -p /data/gitautodev/db /data/gitautodev/storage && chown -R app:app /data/gitautodev

WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/gitautodev /app/gitautodev
COPY --from=builder /app/migrations /app/migrations

# Change ownership
RUN chown -R app:app /app

USER app

EXPOSE 3000

CMD ["./gitautodev"]