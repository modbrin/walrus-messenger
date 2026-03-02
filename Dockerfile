FROM rust:1.93-bookworm AS builder
WORKDIR /app

COPY . .

RUN cargo build --locked --release -p walrus-server

FROM gcr.io/distroless/cc-debian13:nonroot AS runtime

COPY --from=builder /app/target/release/walrus-server /usr/bin/walrus-server

ENTRYPOINT ["/usr/bin/walrus-server"]
