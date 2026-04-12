# BundesarchivUtility

A lightweight web utility for searching and exporting records from the [Bundesarchiv](https://www.bundesarchiv.de) Invenio archive. Built with a Rust/Axum backend and a single-page HTML frontend.

## Features

- **Keyword search** — query the Bundesarchiv Invenio REST API (`/api/records`) by keyword
- **Results view** — displays matching archival records (title, ID, publication date) as styled cards
- **CSV export** — downloads the current search results as a `.csv` file
- **Offline fallback** — returns mock data when no search term is provided or the upstream API is unreachable

## Stack

| Layer    | Technology                          |
|----------|-------------------------------------|
| Backend  | Rust, [Axum](https://github.com/tokio-rs/axum) 0.8, Tokio |
| HTTP client | [reqwest](https://github.com/seanmonstar/reqwest) 0.12 |
| Serialization | serde / serde_json             |
| CSV      | [csv](https://github.com/BurntSushi/rust-csv) 1.3 |
| Frontend | Vanilla HTML/CSS/JS (served as static files via `tower-http`) |

## Prerequisites

- [Rust](https://rustup.rs/) (edition 2024, stable toolchain)

## Getting Started

```bash
# Clone the repository
git clone <repo-url>
cd BundesarchivUtility

# Build and run
cargo run
```

The server starts on **http://127.0.0.1:3000**. Open that URL in your browser to use the UI.

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/query?q=<term>` | Returns matching records as JSON. Omit `q` for mock data. |
| `GET` | `/api/export?q=<term>` | Downloads matching records as `archival_export.csv`. |
| `GET` | `/api/stats` | Returns aggregate digitization statistics (total records, digitized count, percentage). |

### Example

```bash
# Fetch JSON results for "Wehrmacht"
curl "http://127.0.0.1:3000/api/query?q=Wehrmacht"

# Download as CSV
curl -OJ "http://127.0.0.1:3000/api/export?q=Wehrmacht"
```

### JSON response shapes

`/api/query`:
```json
[
  {
    "id": "12345",
    "title": "Historical Document A",
    "publication_date": "1945-05-08"
  }
]
```

`/api/stats`:
```json
{
  "total": 150000,
  "digitized": 42000,
  "percent_digitized": 28.0
}
```

## Project Structure

```
BundesarchivUtility/
├── src/
│   └── main.rs          # Axum server, Invenio API client, CSV export
├── static/
│   └── index.html       # Single-page search UI
└── Cargo.toml
```

## Upstream API

Search queries are forwarded to:

```
https://invenio.bundesarchiv.de/api/records?q=<term>
```

If the upstream request fails, the application falls back to a small set of built-in mock records so the UI remains functional during development or when the archive is unavailable.
