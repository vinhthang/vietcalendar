FROM rust:1.97 AS builder
WORKDIR /usr/src/app


# Pre-fetch and build dependencies for Docker layer caching
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin tests benches config/holidays && \
    echo "fn main() {}" > src/main.rs && \
    echo "fn main() {}" > src/bin/vietcalendar-mcp.rs && \
    echo "pub mod contracts; pub mod kernel; pub mod plugins; pub mod profiles;" > src/lib.rs && \
    touch src/contracts.rs src/kernel.rs src/plugins.rs src/profiles.rs && \
    echo "profiles: {}" > config/cordis.yml && \
    echo "fn main() {}" > tests/integration_test.rs && \
    echo "fn main() {}" > benches/calendar_bench.rs && \
    cargo build --release && \
    rm -rf src tests benches config

# Copy actual source code and config schedules, then build final binaries
COPY config ./config
COPY src ./src
COPY tests ./tests
COPY benches ./benches
RUN touch src/lib.rs src/main.rs src/bin/vietcalendar-mcp.rs && cargo build --release


FROM debian:bookworm-slim
# Install ca-certificates and tzdata
RUN apt-get update && apt-get install -y ca-certificates tzdata && rm -rf /var/lib/apt/lists/*

# Add non-root user
RUN useradd -u 10001 -m -s /bin/sh appuser

COPY --from=builder /usr/src/app/target/release/vietcalendar /usr/local/bin/vietcalendar
COPY --from=builder /usr/src/app/target/release/vietcalendar-mcp /usr/local/bin/vietcalendar-mcp
COPY --from=builder /usr/src/app/config /etc/vietcalendar/config

# Alias for backwards compatibility
RUN ln -s /usr/local/bin/vietcalendar /usr/local/bin/vietcalendar-rs

USER appuser

ENV PORT=8080
ENV TOKIO_WORKER_THREADS=2
ENV RUST_LOG=info
ENV VIETCALENDAR_CONFIG_DIR=/etc/vietcalendar/config

EXPOSE 8080

CMD ["vietcalendar", "serve"]
