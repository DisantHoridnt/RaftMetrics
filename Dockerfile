FROM rust:1.70 as builder

WORKDIR /usr/src/app
COPY . .

RUN cargo build --release

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y libssl1.1 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/RaftMetrics /usr/local/bin/

EXPOSE 8080 8081 8082

CMD ["RaftMetrics"]