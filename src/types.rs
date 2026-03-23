// ============================================================
// AXYOM Auditor v7.0 — types.rs
// COMPATIBILIDAD: tipos idénticos a v6 → hashes históricos válidos
// Michel Antonio Duran Cornejo — Chile 2026
// ============================================================

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TipoFuente {
    Csv,
    Excel,
    Sql,
    Api,
}

/// Orden: Critico > Medio > Bajo (para sort determinista)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severidad {
    Critico,
    Medio,
    Bajo,
}

impl Severidad {
    pub fn desde_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "critico" | "critical" | "crit" => Severidad::Critico,
            "bajo"    | "low"               => Severidad::Bajo,
            _                               => Severidad::Medio,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Severidad::Critico => "CRIT",
            Severidad::Medio   => "MED ",
            Severidad::Bajo    => "BAJO",
        }
    }
}

/// BTreeMap garantiza orden lexicográfico → hash determinista
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entidad {
    pub id:        String,
    pub fuente:    String,
    pub tipo:      TipoFuente,
    pub campos:    BTreeMap<String, String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Anomalia {
    pub regla_id:       String,
    pub descripcion:    String,
    pub entidad_id:     String,
    pub fuente:         String,
    pub campo:          String,
    pub valor_actual:   String,
    pub valor_esperado: String,
    pub normativa:      String,
    pub severidad:      Severidad,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VersionReglas {
    pub conjunto: String,
    pub version:  String,
    pub hash:     String,
}

/// EvidenciaCanonica — DISEÑO IDÉNTICO A v6
///
/// evidence_chain_hash NO es campo de este struct.
/// hash_canonico = sha3(to_string(EvidenciaCanonica))
/// El JSON guardado en disco = to_string(EvidenciaCanonica)
/// → verify: sha3(bytes_archivo) == hash_canonico → PASS siempre
///
/// COMPATIBILIDAD: mantener campos en mismo orden que v6
/// o los hashes históricos de reportes anteriores dejarán de verificar.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenciaCanonica {
    pub audit_id:           String,
    pub version_motor:      String,
    pub timestamp:          String,
    pub fuente:             String,
    pub conexion_redactada: String,
    pub version_reglas:     VersionReglas,
    pub total_entidades:    usize,
    pub total_anomalias:    usize,
    pub anomalias:          Vec<Anomalia>,
    pub hash_input:         String,
    // evidence_chain_hash NO está aquí — ver ResultadoAuditoria
}

#[derive(Debug, Clone)]
pub struct AuditOptions {
    pub fuente:       String,
    pub conexion:     String,
    pub reglas:       String,
    pub timestamp:    Option<String>,
    pub ruta_reglas:  Option<String>,  // v7: ruta a YAML externo
}

#[derive(Debug, Clone)]
pub struct ResultadoAuditoria {
    pub datos:               ResultadoExtraccion,
    pub anomalias:           Vec<Anomalia>,
    pub evidencia:           EvidenciaCanonica,
    pub hash_canonico:       String,
    pub evidence_chain_hash: String,
}

#[derive(Debug, Clone)]
pub struct ResultadoExtraccion {
    pub fuente:    String,
    pub total:     usize,
    pub entidades: Vec<Entidad>,
}

/// Redacta credenciales de una cadena de conexión para el informe público.
pub fn redactar_conexion(conexion: &str) -> String {
    if conexion.contains("://") {
        let scheme_end = conexion.find("://").map(|i| i + 3).unwrap_or(0);
        if let Some(at) = conexion.rfind('@') {
            return format!("{}***{}", &conexion[..scheme_end], &conexion[at..]);
        }
        if let Some(q) = conexion.find('?') {
            return format!("{}?***", &conexion[..q]);
        }
        return conexion.to_string();
    }

    let path = std::path::Path::new(conexion);
    if let Some(name) = path.file_name().and_then(|x| x.to_str()) {
        if conexion.contains('/') || conexion.contains('\\') {
            return format!("**/{}", name);
        }
    }

    let lower = conexion.to_lowercase();
    if lower.contains("password=") || lower.contains("pwd=") || lower.contains("secret=") {
        return "[conexion redactada]".to_string();
    }

    conexion.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redactar_url_credenciales() {
        let r = redactar_conexion("postgresql://user:secreto@localhost:5432/db");
        assert!(!r.contains("secreto"));
        assert!(r.contains("***"));
    }

    #[test]
    fn test_redactar_ruta_absoluta() {
        let r = redactar_conexion("/home/michel/datos.csv");
        assert_eq!(r, "**/datos.csv");
    }

    #[test]
    fn test_redactar_nombre_simple() {
        assert_eq!(redactar_conexion("datos.csv"), "datos.csv");
    }

    #[test]
    fn test_severidad_desde_str() {
        assert_eq!(Severidad::desde_str("critico"), Severidad::Critico);
        assert_eq!(Severidad::desde_str("CRITICAL"), Severidad::Critico);
        assert_eq!(Severidad::desde_str("medio"),    Severidad::Medio);
        assert_eq!(Severidad::desde_str("bajo"),     Severidad::Bajo);
        assert_eq!(Severidad::desde_str("otro"),     Severidad::Medio);
    }
}
