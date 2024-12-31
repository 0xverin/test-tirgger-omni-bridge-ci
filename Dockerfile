FROM rust:1.79 AS builder
WORKDIR /usr/src/tee-bridge
COPY . .
RUN cargo build --release

FROM ubuntu:22.04
COPY --from=builder /usr/src/tee-bridge/target/release/bridge-worker /usr/local/bin/bridge-worker
CMD ["bridge-worker"]