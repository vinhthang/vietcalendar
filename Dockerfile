FROM rust:1.97 AS builder
WORKDIR /usr/src/app


# Pre-fetch and build dependencies for Docker layer caching
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin tests && \
    echo "fn main() {}" > src/main.rs && \
    echo "fn main() {}" > src/bin/vietcalendar-mcp.rs && \
    echo "pub mod calendar; pub mod handlers; pub mod mcp; pub mod models; pub mod services;" > src/lib.rs && \
    touch src/calendar.rs src/handlers.rs src/mcp.rs src/models.rs src/services.rs && \
    echo "fn main() {}" > tests/integration_test.rs && \
    cargo build --release && \
    rm -rf src tests

# Copy actual source code and build final binaries
COPY src ./src
COPY tests ./tests
RUN touch src/lib.rs src/main.rs src/bin/vietcalendar-mcp.rs && cargo build --release


FROM debian:bookworm-slim
# Install ca-certificates and tzdata
RUN apt-get update && apt-get install -y ca-certificates tzdata && rm -rf /var/lib/apt/lists/*

# Add non-root user
RUN useradd -u 10001 -m -s /bin/sh appuser
USER appuser

COPY --from=builder /usr/src/app/target/release/vietcalendar /usr/local/bin/vietcalendar
COPY --from=builder /usr/src/app/target/release/vietcalendar-mcp /usr/local/bin/vietcalendar-mcp

# Alias for backwards compatibility
USER root
RUN ln -s /usr/local/bin/vietcalendar /usr/local/bin/vietcalendar-rs
USER appuser

ENV PORT=8080
ENV TOKIO_WORKER_THREADS=2
ENV RUST_LOG=info

EXPOSE 8080

CMD ["vietcalendar", "serve"]


