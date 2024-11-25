FROM rust:1.71.1-slim-bullseye AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app
COPY . .

# Build with release optimizations
RUN cargo build --release

FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary
COPY --from=builder /usr/src/app/target/release/distributed_analytics_system /usr/local/bin/

# Create data directory
RUN mkdir -p /data && chmod 777 /data

# Set environment variables
ENV RUST_LOG=info \
    RUST_BACKTRACE=1

# Expose API and Raft ports
EXPOSE 8080 8081 8082 9081 9082

CMD ["distributed_analytics_system"]