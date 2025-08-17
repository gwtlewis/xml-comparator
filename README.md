# XML-Compare-API

An async, high-performance REST service written in **Rust + Axum** that compares XML documents.

* ✅  Attribute & content diffing  
* ✅  Path / wildcard ignore rules  
* ✅  Single & batch compare (raw XML or URLs)  
* ✅  Cookie-based authentication + session store  
* ✅  Swagger UI (& OpenAPI 3)  
* ✅  100 % passing unit **and** integration tests  

---

## ⚡ Quick start
```bash
# clone & build
$ git clone <repo>
$ cd xml-comparator
$ cargo run            # runs on http://localhost:3000 by default

# custom port
$ APP_PORT=8080 cargo run
```

Open:  `http://localhost:<PORT>/xml-compare-api/swagger-ui/`

---

## 🌐 Base route
All endpoints are rooted under **/xml-compare-api**.
```
GET  /xml-compare-api/health              – liveness
GET  /xml-compare-api/swagger-ui/         – docs
POST /xml-compare-api/api/…               – API
```

---

## 🔑 Authentication workflow
1.  `POST /api/auth/login` with a target login URL + credentials.  
    • extracts `Set-Cookie` headers & stores them in an in-memory session.  
    • returns `session_id` and cookie list.
2.  Pass that **session_id** in subsequent URL-comparison requests (`session_id` field) – cookies are automatically attached.
3.  `POST /api/auth/logout/{session_id}` to invalidate.
4.  Expired sessions are cleaned every 5 minutes by a background Tokio task.

---

## 📑 Endpoints
| Category | Method | Path | Description |
|----------|--------|------|-------------|
| Health   | GET    | /health | Simple liveness check |
| XML      | POST | /api/compare/xml | Compare two raw XML strings |
| XML-batch| POST | /api/compare/xml/batch | Compare many XML pairs |
| URL      | POST | /api/compare/url | Download two URLs & compare |
| URL-batch| POST | /api/compare/url/batch | Download many URL pairs concurrently |
| Auth     | POST | /api/auth/login | Perform basic‐auth & store cookies |
| Auth     | POST | /api/auth/logout/{id} | Remove session |

All return JSON and `200 OK` on success, structured error JSON otherwise.

---

## 🧮 Ignore rules
* **ignore_properties** – list of attribute keys **or element names** to skip.
* **ignore_paths** – list of element paths.  Supported patterns:
  * Exact – `/root/item`  
  * Prefix – `/root/` (matches anything below)  
  * Wildcard – `/root/item/*` (matches any depth after prefix)

Examples:
```jsonc
{
  "xml1": "<a c=\"1\"><b>foo</b></a>",
  "xml2": "<a c=\"2\"><b>bar</b></a>",
  "ignore_properties": ["c"],          // ignore attribute c
  "ignore_paths": ["/a/b"]             // ignore <b>…</b> content
}
```

---

## 📦 Response schema (success)
```json
{
  "matched": false,
  "match_ratio": 0.5,
  "diffs": [
    {
      "path": "/a",
      "diff_type": "AttributeDifferent", // or ContentDifferent…
      "expected": "c=1",
      "actual":   "c=2",
      "message": "Attribute 'c' differs"
    }
  ],
  "total_elements": 2,
  "matched_elements": 1
}
```

---

## 🏗️  Build / Run / Test
```bash
# debug build & run
cargo run

# production build
cargo build --release

# unit + integration tests (29 total)
cargo test
```

---

## 🗂  Project layout
```
src/
├─ models/        # DTOs & error types
├─ services/      # Business logic (XML diff, HTTP client, auth)
├─ handlers/      # HTTP endpoint handlers
├─ utils/         # Validation & helpers
├─ main.rs        # Binary entry point
└─ lib.rs         # Library entry (for tests)
```

---

## 🚀 Performance notes
* Streaming XML parse with **quick-xml** → low memory.
* Batch endpoints spawn concurrent tasks with **Tokio**.
* HTTP client uses a shared `reqwest::Client` (connection reuse).
* Session cleanup keeps memory footprint constant over time.

---

## ⚖️  License
MIT – see `LICENSE` file.