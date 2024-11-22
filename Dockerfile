FROM rust:1.70.0 AS builder

# Install DuckDB dependencies
RUN apt-get update && apt-get install -y \
    wget \
    unzip \
    build-essential \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# Install DuckDB
RUN wget https://github.com/duckdb/duckdb/archive/refs/tags/v0.9.1.zip \
    && unzip v0.9.1.zip \
    && cd duckdb-0.9.1 \
    && mkdir -p build/release \
    && cd build/release \
    && cmake -DCMAKE_BUILD_TYPE=Release ../.. \
    && cmake --build . \
    && cp src/libduckdb.so /usr/local/lib/ \
    && cp duckdb /usr/local/bin/ \
    && cd ../.. \
    && cd .. \
    && rm -rf duckdb-0.9.1 v0.9.1.zip

WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y \
    libssl1.1 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy DuckDB files
COPY --from=builder /usr/local/lib/libduckdb.so /usr/local/lib/
COPY --from=builder /usr/local/bin/duckdb /usr/local/bin/
RUN ldconfig

COPY --from=builder /usr/src/app/target/release/distributed_analytics_system /usr/local/bin/
COPY --from=builder /usr/src/app/target/release/worker /usr/local/bin/

ENV RUST_LOG=info
EXPOSE 3000 3001

CMD ["distributed_analytics_system"]