// ============================================================
// AXYOM Auditor v7.0 — api.rs
//
// API REST local con Axum.
// Escucha en 127.0.0.1:8080 — solo acceso local, nunca expuesto a internet
// sin proxy/firewall configurado explícitamente.
//
// Endpoints:
//   POST /audit   — auditar un CSV (ruta local al servidor)
//   GET  /health  — estado del servidor
//   GET  /version — versión del motor
//
// Michel Antonio Duran Cornejo — Chile 2026
// ============================================================

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;

use crate::connectors::csv::ConectorCsv;
use crate::core::pipeline::ejecutar_auditoria;
use crate::evidence::VERSION_MOTOR;
use crate::rule_engine::yaml::resolver_reglas;
use crate::types::AuditOptions;

// ─── Estado compartido del servidor ──────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub version: String,
}

// ─── Modelos de request/response ─────────────────────────────

#[derive(Deserialize)]
pub struct AuditRequest {
    /// Ruta local al archivo CSV a auditar
    pub csv: String,
    /// Nombre del conjunto de reglas: "mineria" | "banca" | "legal"
    pub reglas: Option<String>,
    /// Ruta a YAML de reglas personalizado (opcional)
    pub ruta_reglas: Option<String>,
    /// Timestamp RFC3339 fijo para reproducibilidad (opcional)
    pub timestamp: Option<String>,
}

#[derive(Serialize)]
pub struct AuditResponse {
    pub ok:                  bool,
    pub audit_id:            String,
    pub total_entidades:     usize,
    pub total_anomalias:     usize,
    pub hash_canonico:       String,
    pub evidence_chain_hash: String,
    pub anomalias:           Vec<AnomaliaResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error:               Option<String>,
}

#[derive(Serialize)]
pub struct AnomaliaResponse {
    pub regla_id:   String,
    pub severidad:  String,
    pub entidad_id: String,
    pub campo:      String,
    pub actual:     String,
    pub esperado:   String,
    pub normativa:  String,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status:  &'static str,
    pub version: String,
    pub motor:   &'static str,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub ok:    bool,
    pub error: String,
}

// ─── Handlers ─────────────────────────────────────────────────

async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status:  "ok",
        version: state.version.clone(),
        motor:   VERSION_MOTOR,
    })
}

async fn version_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": VERSION_MOTOR,
        "axioma":  "0 = 0",
        "autor":   "Michel Antonio Duran Cornejo",
        "pais":    "Chile",
        "anno":    "2026"
    }))
}

async fn audit_handler(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<AuditRequest>,
) -> Result<Json<AuditResponse>, (StatusCode, Json<ErrorResponse>)> {

    let reglas_nombre = req.reglas.as_deref().unwrap_or("mineria");
    let timestamp     = req.timestamp.clone();

    // Resolver reglas (YAML externo o internas)
    let conjunto = resolver_reglas(reglas_nombre, req.ruta_reglas.as_deref());

    // Construir opciones
    let options = AuditOptions {
        fuente:      "csv".to_string(),
        conexion:    req.csv.clone(),
        reglas:      reglas_nombre.to_string(),
        timestamp:   timestamp,
        ruta_reglas: req.ruta_reglas.clone(),
    };

    // Conectar y auditar
    let ts_str = options.timestamp.clone().unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());
    let conector = ConectorCsv::nuevo(&req.csv, &ts_str);

    match ejecutar_auditoria(&conector, &options, conjunto) {
        Ok(r) => {
            let anomalias = r.anomalias.iter().map(|a| AnomaliaResponse {
                regla_id:   a.regla_id.clone(),
                severidad:  format!("{:?}", a.severidad),
                entidad_id: a.entidad_id.clone(),
                campo:      a.campo.clone(),
                actual:     a.valor_actual.clone(),
                esperado:   a.valor_esperado.clone(),
                normativa:  a.normativa.clone(),
            }).collect();

            Ok(Json(AuditResponse {
                ok:                  true,
                audit_id:            r.evidencia.audit_id,
                total_entidades:     r.datos.total,
                total_anomalias:     r.anomalias.len(),
                hash_canonico:       r.hash_canonico,
                evidence_chain_hash: r.evidence_chain_hash,
                anomalias,
                error: None,
            }))
        }
        Err(e) => Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse { ok: false, error: e }),
        )),
    }
}

// ─── Arranque del servidor ────────────────────────────────────

pub async fn start(puerto: u16) -> Result<(), String> {
    let state = Arc::new(AppState {
        version: VERSION_MOTOR.to_string(),
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/version", get(version_handler))
        .route("/audit",  post(audit_handler))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], puerto));
    eprintln!("[AXYOM API] Escuchando en http://{}", addr);
    eprintln!("[AXYOM API] Endpoints: GET /health, GET /version, POST /audit");

    let listener = tokio::net::TcpListener::bind(addr).await
        .map_err(|e| format!("No se pudo abrir puerto {}: {e}", puerto))?;

    axum::serve(listener, app).await
        .map_err(|e| format!("Error en servidor HTTP: {e}"))?;

    Ok(())
}
