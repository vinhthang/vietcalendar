FROM rust:1.94 as builder
WORKDIR /usr/src/app

# Pre-fetch and build dependencies for Docker layer caching
COPY Cargo.toml Cargo.lock ./
RUN mkdir src tests && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub mod calendar; pub mod handlers; pub mod models; pub mod services;" > src/lib.rs && \
    touch src/calendar.rs src/handlers.rs src/models.rs src/services.rs && \
    echo "fn main() {}" > tests/integration_test.rs && \
    cargo build --release && \
    rm -rf src tests target/release/vietcalendar-rs target/release/deps/vietcalendar_rs*

# Copy actual source code and build final binary
COPY src ./src
COPY tests ./tests
RUN cargo build --release

FROM debian:bookworm-slim
# Install ca-certificates and tzdata
RUN apt-get update && apt-get install -y ca-certificates tzdata && rm -rf /var/lib/apt/lists/*

# Add non-root user
RUN useradd -u 10001 -m -s /bin/sh appuser
USER appuser

COPY --from=builder /usr/src/app/target/release/vietcalendar-rs /usr/local/bin/vietcalendar-rs

ENV PORT=8080
ENV TOKIO_WORKER_THREADS=2
ENV RUST_LOG=info

EXPOSE 8080

CMD ["vietcalendar-rs"]

