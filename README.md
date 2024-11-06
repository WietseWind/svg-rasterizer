# Return rasterized PNG from sanitized and checked SVG URL (Rust)

A secure service that fetches, validates, sanitizes and rasterizes SVG files from URLs. Returns PNGs with configurable dimensions. Features caching, rate limiting and protection against common attack vectors. Written in Rust using actix-web.

## Features

- Fetches SVG from provided URL
- Validates URLs and prevents local network access
- Sanitizes SVG using [svg-hush](https://lib.rs/svg-hush)
- Rasterizes to PNG with configurable dimensions using resvg
- Redis-based caching (24h for valid SVGs, 60s for errors)
- Rate limiting (configurable with Cloudflare IP support)
- Automatic redirect for non-SVG URLs
- Returns PNG with proper content-type headers

## Requirements

- Rust 1.74+ (2021 edition)
- Redis server
- [svg-hush](https://lib.rs/svg-hush) binary installed (`cargo install svg-hush`)
- Port 3000 access (or configure different port)

## Installation

```bash
# Install svg-hush (required)
cargo install svg-hush

# Clone repository
git clone https://github.com/WietseWind/svg-rasterizer
cd svg-rasterizer

# Install dependencies and build
cargo build --release

# Start debug
RUST_LOG=svg_rasterizer=debug,actix_web=debug cargo run

# OR: Start service
cargo run --release
```

## Configuration

Environment variables:
- `PORT`: Server port (default: 3000)
- `REDIS_URL`: Redis connection string (default: redis://localhost:6379)
- `MAX_DIMENSION`: Maximum allowed width/height (default: 4096)
- `RUST_LOG`: Logging level (default: debug), e.g. debug, info, warn

## Usage

### API Endpoint

```
GET /rasterize-svg
```

### Query Parameters

- `url`: (Required) URL of the SVG to process
- `width`: (Optional) Output width in pixels (32-4096, default: 1024)
- `height`: (Optional) Output height in pixels (32-4096, default: 1024)

### Examples

```bash
# Basic usage
curl "http://localhost:3000/rasterize-svg?url=https://example.com/image.svg"

# Custom dimensions
curl "http://localhost:3000/rasterize-svg?url=https://example.com/image.svg&width=800&height=600"
```

### Response Types

The service automatically detects the client type and responds appropriately:

#### Browser/Image Client
- Content-Type: image/png
- Direct PNG image response

#### API Client (curl with non-image Accept header)
```json
{
  "success": true,
  "format": "png",
  "size": 12345,
  "width": 1024,
  "height": 1024,
  "contentType": "image/png",
  "data": "data:image/png;base64,..."
}
```

### Error Handling

- Non-SVG URLs: 400 Bad Request with error message
- Invalid URLs: 400 Bad Request with error message
- Rate limit exceeded: 429 Too Many Requests
- Server errors: 500 Internal Server Error

## Rate Limiting

- 60 requests per 60 seconds per IP (configurable)
- Redis-based rate limiting
- Proper handling of Cloudflare IPs and headers

## Caching

- Successful SVG conversions: 24 hours
- Errors: 60 seconds
- Cache key based on URL and requested dimensions

## Running with systemd

Create a systemd service file `/etc/systemd/system/svg-rasterizer.service`:

```ini
[Unit]
Description=SVG Rasterizer Service
After=network.target

[Service]
Type=simple
User=youruser
WorkingDirectory=/path/to/svg-rasterizer
Environment=RUST_LOG=debug
Environment=PORT=3000
Environment=REDIS_URL=redis://localhost:6379
ExecStart=/path/to/svg-rasterizer/target/release/svg-rasterizer
Restart=always

[Install]
WantedBy=multi-user.target
```

Then:
```bash
sudo systemctl enable svg-rasterizer
sudo systemctl start svg-rasterizer
sudo systemctl status svg-rasterizer
```

## Security

- URL validation prevents local network access
- SVG sanitization removes potentially harmful content (via svg-hush)
- Rate limiting prevents abuse
- Maximum file size limits
- Timeouts on all external requests (10s)
- Memory limits on PNG generation
- Safe SVG to PNG conversion using resvg

## Development

```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run tests
cargo test

# Check formatting
cargo fmt -- --check

# Run linter
cargo clippy
```

## License

MIT

## Contributing

PRs welcome! Please ensure you follow the existing code style and add tests for any new features.

## Performance

The Rust implementation offers several advantages over the Node.js version:
- Lower memory usage
- Better CPU utilization
- Faster processing times
- More predictable performance under load
- Native binary without runtime dependencies