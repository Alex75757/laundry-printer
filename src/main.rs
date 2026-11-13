mod escpos;
mod printer;
mod qr;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json,
    Router,
};
use escpos::{render_receipt, ReceiptRequest};
use printer::{Printer, SharedPrinter};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
struct AppState {
    printer: SharedPrinter,
}

async fn print_receipt(
    State(state): State<AppState>,
    Json(req): Json<ReceiptRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let bytes = render_receipt(&req);

    let mut p = state.printer.lock().await;
    p.print(&bytes).map_err(|e| {
        tracing::error!(error = %e, order_id = %req.order_id, "print failed");
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    tracing::info!(order_id = %req.order_id, "printed");
    Ok(Json(serde_json::json!({
        "ok": true,
        "order_id": req.order_id,
    })))
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok", "service": "laundry-printer"}))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    // ── Хардкод для Windows-теста (COM3, 115200, HW flow) ──
    let port_name = "COM3";
    let printer = Printer::open(port_name)?;
    tracing::info!(
        port = %port_name,
        baud = 115_200,
        "laundry-printer: port opened (115200, 8-N-1, HW flow)"
    );

    let state = AppState {
        printer: Arc::new(Mutex::new(printer)),
    };

    let app = Router::new()
        .route("/print", post(print_receipt))
        .route("/health", get(health))
        .with_state(state);

    // ── Хардкод: 0.0.0.0:3001 (чтобы app в Docker видел) ──
    let addr = "0.0.0.0:3001";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("laundry-printer listening on {}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}