FROM rust:1.83 AS builder
WORKDIR /usr/src/omni-bridge
COPY . .
RUN cargo build --release -p bridge-worker

FROM ubuntu:22.04
COPY --from=builder /usr/src/omni-bridge/target/release/bridge-worker /usr/local/bin/bridge-worker
ENTRYPOINT ["bridge-worker"]