# Railway-optimized Dockerfile - Single stage build
FROM rust:1.83-bookworm

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy project files
COPY . .

# Build the application
RUN cargo build --release

# Copy binary to working directory and make executable
RUN cp ./target/release/web-server-report-websocket ./web-server-report-websocket && \
    chmod +x ./web-server-report-websocket

# Setup runtime user
RUN useradd -ms /bin/bash appuser && \
    chown -R appuser:appuser /app

USER appuser

# Expose port
EXPOSE 8081

# Environment variables
ENV RUST_LOG=info \
    RUST_BACKTRACE=1 \
    HOST="0.0.0.0" \
    PORT="8081"

# Start the application
CMD ["./web-server-report-websocket"]
