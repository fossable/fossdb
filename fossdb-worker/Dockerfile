FROM rust:1.82 as builder

WORKDIR /usr/src/app
COPY Cargo.toml ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    git \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/fossdb-worker /usr/local/bin/fossdb-worker

CMD ["fossdb-worker"]