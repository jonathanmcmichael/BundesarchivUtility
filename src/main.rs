use axum::{
    routing::get,
    Router, Json, response::IntoResponse,
    http::{StatusCode, header},
    extract::Query,
};
use tower_http::services::ServeDir;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::net::SocketAddr;

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
}

/// The top-level response from an Invenio API `/api/records` endpoint.
#[derive(Debug, Deserialize)]
struct InvenioResponse {
    hits: InvenioHits,
}

#[derive(Debug, Deserialize)]
struct InvenioHits {
    hits: Vec<InvenioRecord>,
    /// Total hit count. Classic Invenio returns a plain number; InvenioRDM
    /// returns `{"value": N, "relation": "eq"}`. We use `Value` to handle both.
    total: Option<Value>,
}

/// A specific record in the Invenio system.
#[derive(Debug, Deserialize)]
struct InvenioRecord {
    id: String,
    metadata: InvenioMetadata,
}

#[derive(Debug, Deserialize)]
struct InvenioMetadata {
    title: Option<String>,
    publication_date: Option<String>,
}

/// The flat layout we will export to our CSV.
#[derive(Debug, Serialize, Clone)]
struct CsvEntry {
    id: String,
    title: String,
    publication_date: String,
}

/// Response shape for the /api/stats endpoint.
#[derive(Debug, Serialize)]
struct StatsResponse {
    total: u64,
    digitized: u64,
    percent_digitized: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Bundesarchiv Web Utility starting on http://127.0.0.1:3000");

    let app = Router::new()
        // Serve the beautiful static UI
        .fallback_service(ServeDir::new("static"))
        // API Endpoints
        .route("/api/query", get(fetch_data))
        .route("/api/export", get(export_data))
        .route("/api/stats", get(fetch_stats));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    axum::serve(tokio::net::TcpListener::bind(&addr).await?, app).await?;

    Ok(())
}

async fn fetch_data(Query(params): Query<SearchQuery>) -> Json<Vec<CsvEntry>> {
    let _kw = params.q.unwrap_or_default();
    let mock_json = r#"
    {
      "hits": {
        "hits": [
          {
            "id": "12345",
            "metadata": {
              "title": "Historical Document A",
              "publication_date": "1945-05-08"
            }
          },
          {
            "id": "67890",
            "metadata": {
              "title": "Photograph Collection B",
              "publication_date": "1980-01-01"
            }
          }
        ]
      }
    }
    "#;
    
    // If a search term is provided, we fetch from the real Bundesarchiv API.
    let response_data: InvenioResponse = if !_kw.is_empty() {
        let url = format!("https://invenio.bundesarchiv.de/api/records?q={}", _kw);
        match reqwest::get(&url).await {
            Ok(resp) => {
                let json_response = resp.text().await.unwrap_or_else(|_| "{}".to_string());
                serde_json::from_str(&json_response).unwrap_or_else(|_| serde_json::from_str(mock_json).unwrap())
            },
            Err(_) => serde_json::from_str(mock_json).unwrap(),
        }
    } else {
        // Fallback to mock json if no search
        serde_json::from_str(mock_json).unwrap()
    };
    
    let mut entries = Vec::new();
    for record in response_data.hits.hits {
        entries.push(CsvEntry {
            id: record.id,
            title: record.metadata.title.unwrap_or_else(|| "Unknown".to_string()),
            publication_date: record.metadata.publication_date.unwrap_or_else(|| "Unknown".to_string()),
        });
    }

    Json(entries)
}

/// Extract the total hit count from the polymorphic `hits.total` field.
/// Classic Invenio: `"total": 12345`
/// InvenioRDM:      `"total": {"value": 12345, "relation": "eq"}`
fn extract_total(value: Option<Value>) -> u64 {
    match value {
        Some(Value::Number(n)) => n.as_u64().unwrap_or(0),
        Some(Value::Object(map)) => map
            .get("value")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        _ => 0,
    }
}

async fn fetch_stats() -> Json<StatsResponse> {
    let client = reqwest::Client::new();
    let base = "https://invenio.bundesarchiv.de/api/records";

    // size=0 so we only receive the total, not the records themselves.
    let total = match client.get(format!("{base}?size=0")).send().await {
        Ok(resp) => {
            let parsed: Result<InvenioResponse, _> = resp.json().await;
            extract_total(parsed.ok().and_then(|r| r.hits.total))
        }
        Err(_) => 0,
    };

    // Records that have digitised files attached.
    let digitized = match client
        .get(format!("{base}?size=0&q=_exists_:files"))
        .send()
        .await
    {
        Ok(resp) => {
            let parsed: Result<InvenioResponse, _> = resp.json().await;
            extract_total(parsed.ok().and_then(|r| r.hits.total))
        }
        Err(_) => 0,
    };

    let percent_digitized = if total > 0 {
        (digitized as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    Json(StatsResponse { total, digitized, percent_digitized })
}

async fn export_data(Query(params): Query<SearchQuery>) -> impl IntoResponse {
    // Generate CSV in memory
    let entries = fetch_data(Query(params)).await.0;
    
    let mut wtr = csv::Writer::from_writer(vec![]);
    for entry in entries {
        wtr.serialize(entry).unwrap();
    }
    let csv_data = String::from_utf8(wtr.into_inner().unwrap()).unwrap();

    // Serve as file download
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/csv"), (header::CONTENT_DISPOSITION, "attachment; filename=\"archival_export.csv\"")],
        csv_data
    )
}
