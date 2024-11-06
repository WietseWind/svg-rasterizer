# Build stage
FROM rust:1.74 as builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install svg-hush
RUN cargo install svg-hush

# Create app directory
WORKDIR /app

# Copy source code
COPY . .

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl \
    && rm -rf /var/lib/apt/lists/*

# Copy the svg-hush binary from builder
COPY --from=builder /usr/local/cargo/bin/svg-hush /usr/local/bin/svg-hush

# Copy the application binary from builder
COPY --from=builder /app/target/release/svg-rasterizer /usr/local/bin/svg-rasterizer

# Set the startup command
CMD ["svg-rasterizer"]
