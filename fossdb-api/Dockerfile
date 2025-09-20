FROM rust:1.82 as builder

WORKDIR /usr/src/app
COPY Cargo.toml ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/fossdb-api /usr/local/bin/fossdb-api

EXPOSE 3000
CMD ["fossdb-api"]