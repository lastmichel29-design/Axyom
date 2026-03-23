// ============================================================
// AXYOM Auditor v7.0 — lib.rs
//
// Mejoras sobre v6:
//   + JSON canónico → hash determinista en cualquier plataforma
//   + Reglas desde YAML externo → cambio normativo sin recompilar
//   + Streaming CSV → datasets de millones de filas
//   + Paralelismo rayon → evaluación multi-core
//   + API REST local con Axum
//   + rust-toolchain.toml → reproducibilidad total
//
// Compatibilidad v6:
//   + EvidenciaCanonica: mismos campos → reportes históricos siguen verificando
//   + CLI: mismos comandos → axyom audit / verify / info
//   + Conectores: mismas fuentes → CSV, Excel, SQL, API
//
// Axioma: 0 = 0 | Michel Antonio Durán Cornejo — Chile 2026
// ============================================================

pub mod api;
pub mod canonical;
pub mod connectors;
pub mod core;
pub mod error;
pub mod evidence;
pub mod hash;
pub mod rule_engine;
pub mod types;

// Re-exportar lo más usado públicamente
pub use core::pipeline::ejecutar_auditoria;
pub use evidence::{generar_evidencia, verificar_integridad, VERSION_MOTOR};
pub use types::{AuditOptions, EvidenciaCanonica, ResultadoAuditoria};
