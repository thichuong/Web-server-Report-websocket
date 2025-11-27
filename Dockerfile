# Stage 1: Builder
# Sử dụng bookworm cho builder để khớp với runtime (Debian 12)
FROM rust:slim-bookworm as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Caching layer for dependencies
# Copy Cargo.toml and Cargo.lock first to cache dependencies
COPY Cargo.toml Cargo.lock ./
# Create a dummy main.rs to build dependencies
# Create dummy files to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    echo "" > src/lib.rs

# Build dependencies only
RUN cargo build --release && \
    rm -rf src target/release/deps/web_server_report_websocket* target/release/deps/libweb_server_report_websocket*

# Build the actual application
COPY . .
RUN cargo build --release

# --- KHẮC PHỤC LỖI CHMOD Ở ĐÂY ---
# Copy file ra một chỗ và cấp quyền ngay tại builder stage (nơi có shell và chmod)
# Distroless image không có shell, nên phải chmod trước khi copy vào đó
RUN cp target/release/web-server-report-websocket ./web-server-report-websocket && \
    chmod +x ./web-server-report-websocket

# Stage 2: Runtime
# Sử dụng gcr.io/distroless/cc-debian12 cho image nhỏ gọn và bảo mật
FROM gcr.io/distroless/cc-debian12

WORKDIR /app

# Copy binary đã được chmod từ builder
COPY --from=builder /app/web-server-report-websocket ./web-server-report-websocket

# Copy assets
# Lưu ý: Các file này được copy nguyên trạng từ source code
# COPY --from=builder /app/dashboards ./dashboards
# COPY --from=builder /app/shared_components ./shared_components
# COPY --from=builder /app/shared_assets ./shared_assets

# User: Distroless mặc định chạy với user nonroot (uid 65532)
# Không cần lệnh USER hay useradd, nhưng cần đảm bảo app lắng nghe ở port > 1024 nếu không có capability.
# Tuy nhiên port 8000 là an toàn.
USER nonroot

EXPOSE 8001

ENV RUST_LOG=info \
    HOST="0.0.0.0"

CMD ["./web-server-report-websocket"]
