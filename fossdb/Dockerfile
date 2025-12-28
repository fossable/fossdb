FROM rust:1.75 as builder

WORKDIR /app
COPY Cargo.toml ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Create data directory for native_db
RUN mkdir -p /data

COPY --from=builder /app/target/release/fossdb /app/fossdb

ENV DATABASE_PATH=/data/fossdb.db
ENV SERVER_PORT=3000

EXPOSE 3000

CMD ["./fossdb"]
