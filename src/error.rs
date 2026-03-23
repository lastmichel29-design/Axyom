// ============================================================
// AXYOM Auditor v7.0 — error.rs
// Errores tipados con thiserror — sin unwrap() en código core
// Michel Antonio Duran Cornejo — Chile 2026
// ============================================================

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AxyomError {
    #[error("Error de conexión/lectura: {0}")]
    Connector(String),

    #[error("Error cargando reglas YAML '{archivo}': {detalle}")]
    ReglasYaml { archivo: String, detalle: String },

    #[error("Regla inválida '{id}': {motivo}")]
    ReglaInvalida { id: String, motivo: String },

    #[error("Error de serialización JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Error de IO '{ruta}': {detalle}")]
    Io { ruta: String, detalle: String },

    #[error("Error generando PDF: {0}")]
    Pdf(String),

    #[error("Query no permitida: {0}")]
    QueryProhibida(String),

    #[error("Error en servidor HTTP: {0}")]
    Http(String),

    #[error("Hash de verificación inválido: esperado={esperado} calculado={calculado}")]
    HashMismatch { esperado: String, calculado: String },
}

impl AxyomError {
    pub fn connector(msg: impl Into<String>) -> Self {
        AxyomError::Connector(msg.into())
    }
    pub fn io(ruta: impl Into<String>, e: impl std::fmt::Display) -> Self {
        AxyomError::Io { ruta: ruta.into(), detalle: e.to_string() }
    }
    pub fn pdf(msg: impl Into<String>) -> Self {
        AxyomError::Pdf(msg.into())
    }
}

pub type AxyomResult<T> = Result<T, AxyomError>;
