# XML Comparison REST API

A high-performance REST API for comparing XML documents built with Rust and Axum.

## Features

- **Basic XML Comparison**: Compare two XMLs with ignore paths/properties support
- **Batch Processing**: High-performance concurrent comparison of multiple XML pairs
- **URL-Based Comparison**: Download and compare XMLs from URLs
- **Async Batch URL Processing**: Process multiple URL pairs with concurrent downloads
- **Authentication Support**: Basic auth with session management and cookie support
- **OpenAPI Documentation**: Complete API specification with Swagger UI
- **High Performance**: Built with Rust and Axum for maximum speed and memory efficiency

## Quick Start

### Prerequisites

- Rust 1.70+ installed
- Cargo package manager

### Installation & Running

```bash
# Clone the repository
git clone <repository-url>
cd xml-comparison-api

# Build and run
cargo run

# The server will start on http://localhost:8080
```

### API Endpoints

- `GET /api/v1/health` - Health check
- `POST /api/v1/compare` - Compare two XML strings
- `POST /api/v1/compare/batch` - Compare multiple XML pairs
- `POST /api/v1/compare/urls` - Compare XMLs from URLs
- `POST /api/v1/compare/urls/batch` - Compare multiple XML URL pairs
- `POST /api/v1/auth/login` - Authenticate for protected URLs
- `GET /api/v1/auth/status` - Check authentication status
- `GET /api-docs/openapi.json` - OpenAPI specification

## Example Usage

### Basic XML Comparison

```bash
curl -X POST http://localhost:8080/api/v1/compare \
  -H "Content-Type: application/json" \
  -d '{
    "xml1": "<root><item>value1</item></root>",
    "xml2": "<root><item>value2</item></root>",
    "ignore_paths": [],
    "ignore_properties": []
  }'
```

### Health Check

```bash
curl http://localhost:8080/api/v1/health
```

## Development

### Building

```bash
# Development build
cargo build

# Release build
cargo build --release
```

### Running Tests

```bash
cargo test
```

### API Documentation

The OpenAPI specification is available at `http://localhost:8080/api-docs/openapi.json` when the server is running.

## Architecture

- **Framework**: Axum for async web server
- **Runtime**: Tokio for async operations
- **XML Processing**: quick-xml and xmltree for parsing
- **HTTP Client**: reqwest with rustls for secure connections
- **Documentation**: utoipa for OpenAPI generation
- **Authentication**: Custom session management with dashmap
- **Validation**: Comprehensive input validation

## License

This project is licensed under the MIT License.