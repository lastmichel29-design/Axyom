// ============================================================
// AXYOM Auditor v7.0 — core/pipeline.rs
//
// Pipeline de auditoría:
//   Fuente → Extracción → Canonización → Reglas → Decisiones → Evidencia → SHA3
//
// MEJORA v7: paralelismo con rayon para datasets grandes.
//   v6: evaluación secuencial O(n * m)
//   v7: evaluación paralela O(n * m / cores) con rayon::par_iter()
//
// La paralelización es segura porque:
//   - Cada entidad se evalúa independientemente (sin estado compartido mutable)
//   - El resultado se canoniza después de recolectar → orden determinista
//
// Michel Antonio Duran Cornejo — Chile 2026
// ============================================================

use rayon::prelude::*;

use crate::canonical::canonical_str;
use crate::connectors::ConectorReadonly;
use crate::evidence::{canonizar_anomalias, canonizar_entidades, VERSION_MOTOR};
use crate::hash::sha3_str;
use crate::rule_engine::{ConjuntoReglas, MotorReglas};
use crate::types::{AuditOptions, EvidenciaCanonica, ResultadoAuditoria, redactar_conexion};

pub fn ejecutar_auditoria<C: ConectorReadonly>(
    conector: &C,
    opciones: &AuditOptions,
    conjunto:  ConjuntoReglas,
) -> Result<ResultadoAuditoria, String> {

    // ── 1. Extraer datos ──────────────────────────────────────
    let mut datos = conector.extraer()?;

    // ── 2. Canonizar entidades: orden determinista ────────────
    canonizar_entidades(&mut datos.entidades);

    // ── 3. Versión de reglas (hash del conjunto) ──────────────
    let version_reglas = conjunto.version_reglas();

    // ── 4. Timestamp ──────────────────────────────────────────
    //    Controlado externamente → reproducible en tests
    //    Fijo por defecto si no se pasa → hash determinista
    let timestamp = opciones
        .timestamp
        .clone()
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());

    // ── 5. Evaluar reglas en paralelo (rayon) ─────────────────
    //    Cada entidad se evalúa en su propio hilo.
    //    Collect reúne resultados en Vec no ordenado → canonizar después.
    let motor = MotorReglas::nuevo(conjunto);

    let mut anomalias: Vec<_> = datos.entidades
        .par_iter()
        .flat_map(|entidad| {
            let mut parcel = Vec::new();
            for regla in &motor.conjunto.simples {
                if let Some(a) = crate::rule_engine::evaluar_simple_pub(regla, entidad) {
                    parcel.push(a);
                }
            }
            for regla in &motor.conjunto.condicionales {
                if let Some(a) = crate::rule_engine::evaluar_condicional_pub(regla, entidad) {
                    parcel.push(a);
                }
            }
            parcel
        })
        .collect();

    // ── 6. Canonizar anomalías → orden determinista ───────────
    canonizar_anomalias(&mut anomalias);

    // ── 7. hash_input: SHA3 sobre entidades canonizadas ───────
    //    Serialización de entidades usa BTreeMap → orden estable
    let entidades_json = serde_json::to_string(&datos.entidades)
        .map_err(|e| format!("Error serializando entidades: {e}"))?;
    let hash_input = sha3_str(&entidades_json);

    // ── 8. Conexión redactada para evidencia pública ──────────
    let conexion_redactada = redactar_conexion(&opciones.conexion);

    // ── 9. Audit ID determinista (derivado de los datos) ──────
    //    No usa uuid/random → mismo input → mismo audit_id
    let audit_basis = serde_json::json!({
        "version_motor":      VERSION_MOTOR,
        "fuente":             opciones.fuente,
        "conexion_redactada": conexion_redactada,
        "version_reglas":     version_reglas,
        "hash_input":         hash_input,
        "timestamp":          timestamp,
    });
    let audit_id = format!(
        "audit-{}",
        &sha3_str(&serde_json::to_string(&audit_basis).unwrap())[..24]
    );

    // ── 10. Construir EvidenciaCanonica ───────────────────────
    //     evidence_chain_hash NO está aquí — ver ResultadoAuditoria
    let evidencia = EvidenciaCanonica {
        audit_id:           audit_id.clone(),
        version_motor:      VERSION_MOTOR.to_string(),
        timestamp:          timestamp.clone(),
        fuente:             opciones.fuente.clone(),
        conexion_redactada: conexion_redactada.clone(),
        version_reglas:     version_reglas.clone(),
        total_entidades:    datos.total,
        total_anomalias:    anomalias.len(),
        anomalias:          anomalias.clone(),
        hash_input:         hash_input.clone(),
    };

    // ── 11. hash_canonico = sha3(canonical_json(evidencia)) ───
    //     MEJORA v7: canonical_str garantiza orden de claves
    //     → hash idéntico en cualquier plataforma
    let json_val      = serde_json::to_value(&evidencia)
        .map_err(|e| format!("Error convirtiendo evidencia a Value: {e}"))?;
    let evidencia_canonical = canonical_str(&json_val);
    let hash_canonico = sha3_str(&evidencia_canonical);

    // ── 12. evidence_chain_hash: metadata de encadenamiento ───
    //     Fuera de EvidenciaCanonica → no afecta hash_canonico
    let evidence_chain_hash = sha3_str(
        &format!("{}|{}|{}|{}",
            audit_id,
            hash_canonico,
            hash_input,
            serde_json::to_string(&version_reglas).unwrap_or_default()
        )
    );

    Ok(ResultadoAuditoria {
        datos,
        anomalias,
        evidencia,
        hash_canonico,
        evidence_chain_hash,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectors::csv::ConectorCsv;
    use crate::evidence::verificar_integridad;
    use crate::rule_engine::yaml::cargar_desde_str;
    use std::io::Write;

    const TS: &str = "1970-01-01T00:00:00Z";

    const YAML_TEST: &str = r#"
meta:
  conjunto: "mineria"
  version: "3.0.0"
simples:
  - id: "M001"
    descripcion: "EPP obligatorio"
    campo: "epp_asignado"
    condicion: "no_vacio"
    normativa: "DS 44/2024"
    severidad: "critico"
  - id: "M003"
    descripcion: "Examen medico aprobado"
    campo: "examen_medico"
    condicion: "igual_a"
    valor: "APROBADO"
    normativa: "DS 594"
    severidad: "critico"
condicionales: []
"#;

    fn csv_fixture() -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "nombre,epp_asignado,fecha_contrato,examen_medico,capacitacion_seguridad").unwrap();
        writeln!(f, "Juan,CASCO,2024-01-15,APROBADO,COMPLETADA").unwrap();
        writeln!(f, "Maria,,2024-02-01,APROBADO,COMPLETADA").unwrap();
        writeln!(f, "Carlos,CASCO,2024-03-10,PENDIENTE,COMPLETADA").unwrap();
        f
    }

    fn opts(f: &tempfile::NamedTempFile) -> AuditOptions {
        AuditOptions {
            fuente:      "csv".into(),
            conexion:    f.path().to_string_lossy().to_string(),
            reglas:      "mineria".into(),
            timestamp:   Some(TS.into()),
            ruta_reglas: None,
        }
    }

    #[test]
    fn test_pipeline_completo() {
        let f  = csv_fixture();
        let c  = ConectorCsv::nuevo(f.path(), TS);
        let cj = cargar_desde_str(YAML_TEST, "test").unwrap();
        let r  = ejecutar_auditoria(&c, &opts(&f), cj).unwrap();

        assert_eq!(r.datos.total, 3);
        assert!(!r.anomalias.is_empty());
        assert!(!r.hash_canonico.is_empty());
        assert_eq!(r.hash_canonico.len(), 64);
        assert!(!r.evidence_chain_hash.is_empty());
        assert_ne!(r.hash_canonico, r.evidence_chain_hash);
    }

    #[test]
    fn test_hash_reproducible() {
        let f  = csv_fixture();
        let c  = ConectorCsv::nuevo(f.path(), TS);
        let cj1 = cargar_desde_str(YAML_TEST, "test").unwrap();
        let cj2 = cargar_desde_str(YAML_TEST, "test").unwrap();
        let r1 = ejecutar_auditoria(&c, &opts(&f), cj1).unwrap();
        let r2 = ejecutar_auditoria(&c, &opts(&f), cj2).unwrap();

        assert_eq!(r1.hash_canonico, r2.hash_canonico,
            "Mismo input + timestamp fijo → hash idéntico");
        assert_eq!(r1.audit_id, r2.audit_id,
            "audit_id también debe ser determinista");
    }

    #[test]
    fn test_cadena_hash_cierra_con_export() {
        let f  = csv_fixture();
        let c  = ConectorCsv::nuevo(f.path(), TS);
        let cj = cargar_desde_str(YAML_TEST, "test").unwrap();
        let r  = ejecutar_auditoria(&c, &opts(&f), cj).unwrap();

        let dir  = tempfile::tempdir().unwrap();
        let pref = dir.path().join("reporte").to_string_lossy().to_string();

        let (_pdf, json) = crate::evidence::generar_evidencia(
            &r.evidencia, &r.hash_canonico, &pref
        ).unwrap();

        let ok = verificar_integridad(&json, &r.hash_canonico).unwrap();
        assert!(ok, "verify debe pasar siempre después de generar_evidencia");
    }

    #[test]
    fn test_anomalias_juan_sin_anomalias() {
        // Solo Juan en el CSV
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "nombre,epp_asignado,examen_medico").unwrap();
        writeln!(f, "Juan,CASCO,APROBADO").unwrap();

        let c  = ConectorCsv::nuevo(f.path(), TS);
        let cj = cargar_desde_str(YAML_TEST, "test").unwrap();
        let op = AuditOptions {
            fuente: "csv".into(), conexion: f.path().to_string_lossy().to_string(),
            reglas: "mineria".into(), timestamp: Some(TS.into()), ruta_reglas: None,
        };
        let r = ejecutar_auditoria(&c, &op, cj).unwrap();
        assert_eq!(r.anomalias.len(), 0, "Juan no debe tener anomalías");
    }
}
