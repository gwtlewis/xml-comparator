# XML Compare API

A high-performance REST API for comparing XML documents with support for URL-based comparisons, batch processing, and authentication.

## Features

### 1. XML Content Comparison
- Compare two XML documents by their contents
- Support for ignoring specific paths or properties
- Returns detailed diff information and match ratio
- High-performance comparison engine

### 2. Batch XML Comparison
- Compare multiple XML pairs efficiently
- Parallel processing for improved performance
- Comprehensive batch results with success/failure statistics

### 3. URL-based XML Comparison
- Download and compare XMLs from URLs
- Support for authenticated URLs
- Automatic cookie management for authenticated requests

### 4. Batch URL Comparison
- Compare XMLs from multiple URL pairs
- Async I/O for improved performance
- Concurrent downloads with session management

### 5. Authentication System
- Basic authentication for protected URLs
- Session management with cookie handling
- Automatic session cleanup

## API Endpoints

### XML Comparison
- `POST /api/compare/xml` - Compare two XML contents
- `POST /api/compare/xml/batch` - Compare multiple XML pairs

### URL Comparison
- `POST /api/compare/url` - Compare XMLs from two URLs
- `POST /api/compare/url/batch` - Compare XMLs from multiple URL pairs

### Authentication
- `POST /api/auth/login` - Authenticate with a URL
- `POST /api/auth/logout/{session_id}` - Logout and invalidate session

### Documentation
- `GET /swagger-ui` - Interactive API documentation

## Installation

1. Clone the repository:
```bash
git clone <repository-url>
cd xml-compare-api
```

2. Install Rust (if not already installed):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

3. Build and run the application:
```bash
cargo run
```

The server will start on `http://localhost:3000`

## Usage Examples

### Compare Two XML Contents

```bash
curl -X POST http://localhost:3000/api/compare/xml \
  -H "Content-Type: application/json" \
  -d '{
    "xml1": "<a c=\"C\"><child>hey</child></a>",
    "xml2": "<a c=\"D\"><child>hey</child></a>",
    "ignore_properties": ["c"]
  }'
```

### Compare XMLs from URLs

```bash
curl -X POST http://localhost:3000/api/compare/url \
  -H "Content-Type: application/json" \
  -d '{
    "url1": "https://example.com/xml1.xml",
    "url2": "https://example.com/xml2.xml",
    "ignore_paths": ["/root/timestamp"],
    "auth_credentials": {
      "username": "user",
      "password": "pass"
    }
  }'
```

### Authenticate with a URL

```bash
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://example.com/login",
    "username": "user",
    "password": "pass"
  }'
```

### Batch XML Comparison

```bash
curl -X POST http://localhost:3000/api/compare/xml/batch \
  -H "Content-Type: application/json" \
  -d '{
    "comparisons": [
      {
        "xml1": "<a>test1</a>",
        "xml2": "<a>test1</a>"
      },
      {
        "xml1": "<b>test2</b>",
        "xml2": "<b>test3</b>"
      }
    ]
  }'
```

## Ignoring Properties and Paths

### Ignoring Properties
When you specify properties to ignore, the comparison will skip those attributes or element names:

```xml
<!-- XML 1 -->
<a c="C"><child>hey</child></a>

<!-- XML 2 -->
<a c="D"><child>yo</child></a>
```

With `"ignore_properties": ["c"]` - the `c` attribute will be ignored
With `"ignore_properties": ["child"]` - the `child` element content will be ignored

### Ignoring Paths
When you specify paths to ignore, any elements matching those paths will be skipped:

```xml
<!-- XML 1 -->
<root><timestamp>2023-01-01</timestamp><data>value1</data></root>

<!-- XML 2 -->
<root><timestamp>2023-01-02</timestamp><data>value1</data></root>
```

With `"ignore_paths": ["/root/timestamp"]` - the timestamp element will be ignored

## Response Format

### Successful Comparison Response
```json
{
  "matched": true,
  "match_ratio": 1.0,
  "diffs": [],
  "total_elements": 2,
  "matched_elements": 2
}
```

### Failed Comparison Response
```json
{
  "matched": false,
  "match_ratio": 0.5,
  "diffs": [
    {
      "path": "/a",
      "diff_type": "AttributeDifferent",
      "expected": "c=C",
      "actual": "c=D",
      "message": "Attribute 'c' differs"
    }
  ],
  "total_elements": 2,
  "matched_elements": 1
}
```

## Development

### Running Tests
```bash
cargo test
```

### Building for Production
```bash
cargo build --release
```

### Code Structure
```
src/
├── models/          # Data structures and types
├── services/        # Business logic
├── handlers/        # HTTP request handlers
├── utils/           # Utility functions
└── main.rs          # Application entry point
```

## Configuration

The application runs on port 3000 by default. You can modify the port in `src/main.rs`.

## Performance Considerations

- The XML comparison engine is optimized for performance
- Batch operations use concurrent processing
- URL downloads are performed asynchronously
- Session cleanup runs automatically every hour

## Error Handling

The API provides comprehensive error handling:
- HTTP 400 for invalid requests
- HTTP 401 for authentication failures
- HTTP 500 for internal server errors
- Detailed error messages in JSON format

## License

This project is licensed under the MIT License.