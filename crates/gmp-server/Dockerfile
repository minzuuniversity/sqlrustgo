# GMP Server Dockerfile
# Multi-stage build for optimized production image

FROM rust:1.80 AS builder

WORKDIR /build

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy GMP-Platform workspace
COPY Cargo.toml Cargo.lock ./
COPY crates/gmp-server ./crates/gmp-server/

# Copy sqlrustgo dependency (pre-built or cloned)
RUN mkdir -p /build/../dev/yinglichina163/sqlrustgo/crates/mysql-server
COPY dev/yinglichina163/sqlrustgo/Cargo.toml /build/../dev/yinglichina163/sqlrustgo/
COPY dev/yinglichina163/sqlrustgo/crates/mysql-server /build/../dev/yinglichina163/sqlrustgo/crates/mysql-server/

# Build dependencies first (for caching)
RUN cargo build -p gmp-server --release 2>/dev/null || true

# Build the final binary
RUN cargo build -p gmp-server --release

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/gmp-server /app/gmp-server

# Copy default config
COPY --from=builder /build/crates/gmp-server/config /app/config

# Create data directory
RUN mkdir -p /var/lib/gmp-server/data

# Environment variables
ENV RUST_LOG=info
ENV GMP_SERVER_HOST=0.0.0.0
ENV GMP_SERVER_PORT=3306

# Expose MySQL port
EXPOSE 3306 8080

# Run the server
CMD ["/app/gmp-server", "--config", "/app/config/default.toml"]
