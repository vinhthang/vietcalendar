FROM rust:1.94 as builder
WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
# Install ca-certificates and tzdata which are useful for web/api apps
RUN apt-get update && apt-get install -y ca-certificates tzdata && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/app/target/release/vietcalendar-rs /usr/local/bin/vietcalendar-rs

ENV PORT=8080
ENV TOKIO_WORKER_THREADS=2
ENV RUST_LOG=info

EXPOSE ${PORT}

CMD ["vietcalendar-rs"]
