// ============================================================
// AXYOM Auditor v7.0 — tests/integration.rs
//
// Tests end-to-end que prueban el sistema completo:
//   CSV → Pipeline → Evidencia → SHA3 → Verify
//
// Todos los tests usan timestamp fijo "1970-01-01T00:00:00Z"
// → resultados deterministas y reproducibles.
//
// Michel Antonio Durán Cornejo — Chile 2026
// ============================================================

use std::io::Write;
use axyom::{
    connectors::csv::ConectorCsv,
    core::pipeline::ejecutar_auditoria,
    evidence::{generar_evidencia, verificar_integridad, VERSION_MOTOR},
    rule_engine::yaml::{cargar_desde_str, resolver_reglas},
    types::AuditOptions,
};

const TS: &str = "1970-01-01T00:00:00Z";

// ─── CSV de prueba (mismo que demo_trabajadores.csv) ──────────

const CSV_COMPLETO: &str = "\
nombre,rut_trabajador,epp_asignado,fecha_contrato,examen_medico,capacitacion_seguridad
Juan Perez,12345678-9,CASCO-GUANTES,2024-01-15,APROBADO,COMPLETADA
Maria Lopez,98765432-1,,2024-02-01,APROBADO,COMPLETADA
Carlos Soto,11223344-5,CASCO,2024-03-10,PENDIENTE,COMPLETADA
Ana Torres,55667788-2,CASCO-GUANTES,,APROBADO,
Pedro Rojas,99887766-3,CASCO,2024-04-05,APROBADO,COMPLETADA
Luis Vera,44332211-7,CASCO-ZAPATOS,2024-04-10,APROBADO,COMPLETADA
Rosa Munoz,77665544-K,,2024-05-01,APROBADO,
";

fn csv_temp(contenido: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(contenido.as_bytes()).unwrap();
    f
}

fn opts(ruta: &str) -> AuditOptions {
    AuditOptions {
        fuente:      "csv".into(),
        conexion:    ruta.into(),
        reglas:      "mineria".into(),
        timestamp:   Some(TS.into()),
        ruta_reglas: None,
    }
}

// ─── Tests principales ────────────────────────────────────────

#[test]
fn test_version_motor() {
    assert_eq!(VERSION_MOTOR, "7.0.0");
}

#[test]
fn test_pipeline_7_trabajadores_10_anomalias() {
    let f   = csv_temp(CSV_COMPLETO);
    let con = ConectorCsv::nuevo(f.path(), TS);
    let cj  = resolver_reglas("mineria", None);
    let r   = ejecutar_auditoria(&con, &opts(f.path().to_str().unwrap()), cj).unwrap();

    assert_eq!(r.datos.total, 7, "Deben leerse 7 trabajadores");
    assert_eq!(r.anomalias.len(), 10, "Demo debe producir exactamente 10 anomalías");

    // Contar por severidad
    let criticas = r.anomalias.iter()
        .filter(|a| matches!(a.severidad, axyom::types::Severidad::Critico))
        .count();
    let medias = r.anomalias.iter()
        .filter(|a| matches!(a.severidad, axyom::types::Severidad::Medio))
        .count();

    assert!(criticas > 0, "Debe haber al menos una anomalía crítica");
    assert!(medias   > 0, "Debe haber al menos una anomalía media");
    assert_eq!(criticas + medias, 10);
}

#[test]
fn test_hash_canonico_reproducible_doble_ejecucion() {
    let f    = csv_temp(CSV_COMPLETO);
    let ruta = f.path().to_str().unwrap();
    let con  = ConectorCsv::nuevo(f.path(), TS);

    let cj1 = resolver_reglas("mineria", None);
    let cj2 = resolver_reglas("mineria", None);
    let r1  = ejecutar_auditoria(&con, &opts(ruta), cj1).unwrap();
    let r2  = ejecutar_auditoria(&con, &opts(ruta), cj2).unwrap();

    assert_eq!(r1.hash_canonico, r2.hash_canonico,
        "hash_canonico debe ser idéntico entre ejecuciones con mismo input");
    assert_eq!(r1.audit_id, r2.audit_id,
        "audit_id también debe ser determinista");
    assert_eq!(r1.anomalias.len(), r2.anomalias.len());
}

#[test]
fn test_cadena_hash_cierra_end_to_end() {
    let f    = csv_temp(CSV_COMPLETO);
    let ruta = f.path().to_str().unwrap();
    let con  = ConectorCsv::nuevo(f.path(), TS);
    let cj   = resolver_reglas("mineria", None);
    let r    = ejecutar_auditoria(&con, &opts(ruta), cj).unwrap();

    let dir  = tempfile::tempdir().unwrap();
    let pref = dir.path().join("reporte_int").to_string_lossy().to_string();

    let (pdf, json) = generar_evidencia(&r.evidencia, &r.hash_canonico, &pref).unwrap();

    // Archivos deben existir
    assert!(std::path::Path::new(&pdf).exists(),  "PDF debe existir");
    assert!(std::path::Path::new(&json).exists(), "JSON debe existir");

    // LA PRUEBA CENTRAL: verify debe pasar siempre
    let ok = verificar_integridad(&json, &r.hash_canonico).unwrap();
    assert!(ok, "verificar_integridad debe pasar después de generar_evidencia");
}

#[test]
fn test_hash_difiere_con_datos_distintos() {
    let csv1 = "nombre,epp_asignado\nJuan,CASCO\n";
    let csv2 = "nombre,epp_asignado\nMaria,\n";

    let f1 = csv_temp(csv1);
    let f2 = csv_temp(csv2);

    let c1 = ConectorCsv::nuevo(f1.path(), TS);
    let c2 = ConectorCsv::nuevo(f2.path(), TS);

    let cj1 = resolver_reglas("mineria", None);
    let cj2 = resolver_reglas("mineria", None);

    let r1 = ejecutar_auditoria(&c1, &opts(f1.path().to_str().unwrap()), cj1).unwrap();
    let r2 = ejecutar_auditoria(&c2, &opts(f2.path().to_str().unwrap()), cj2).unwrap();

    assert_ne!(r1.hash_canonico, r2.hash_canonico,
        "Datos distintos deben producir hashes distintos");
}

#[test]
fn test_juan_sin_anomalias() {
    let csv = "nombre,epp_asignado,fecha_contrato,examen_medico,capacitacion_seguridad\n\
               Juan,CASCO,2024-01-15,APROBADO,COMPLETADA\n";
    let f   = csv_temp(csv);
    let con = ConectorCsv::nuevo(f.path(), TS);
    let cj  = resolver_reglas("mineria", None);
    let r   = ejecutar_auditoria(&con, &opts(f.path().to_str().unwrap()), cj).unwrap();
    assert_eq!(r.anomalias.len(), 0, "Juan no debe tener anomalías");
    assert_eq!(r.datos.total, 1);
}

#[test]
fn test_maria_con_anomalias_m001_mc002() {
    let csv = "nombre,epp_asignado,fecha_contrato,examen_medico,capacitacion_seguridad\n\
               Maria,,2024-02-01,APROBADO,COMPLETADA\n";
    let f   = csv_temp(csv);
    let con = ConectorCsv::nuevo(f.path(), TS);
    let cj  = resolver_reglas("mineria", None);
    let r   = ejecutar_auditoria(&con, &opts(f.path().to_str().unwrap()), cj).unwrap();

    let ids: Vec<&str> = r.anomalias.iter().map(|a| a.regla_id.as_str()).collect();
    assert!(ids.contains(&"M001"), "Maria debe tener M001 (sin EPP)");
    assert!(ids.contains(&"MC002"), "Maria debe tener MC002 (contrato sin EPP)");
}

#[test]
fn test_reglas_desde_yaml_externo() {
    let yaml = r#"
meta:
  conjunto: "test_int"
  version:  "1.0.0"
simples:
  - id: "TX01"
    descripcion: "Campo test no vacío"
    campo: "campo_test"
    condicion: "no_vacio"
    normativa: "Test"
    severidad: "critico"
condicionales: []
"#;
    let csv = "campo_test\n\n";  // Una fila con campo vacío
    let f   = csv_temp(csv);
    let cj  = cargar_desde_str(yaml, "test").unwrap();

    assert_eq!(cj.nombre,  "test_int");
    assert_eq!(cj.version, "1.0.0");

    let con = ConectorCsv::nuevo(f.path(), TS);
    let op  = AuditOptions {
        fuente: "csv".into(), conexion: f.path().to_string_lossy().into(),
        reglas: "test_int".into(), timestamp: Some(TS.into()), ruta_reglas: None,
    };
    let r = ejecutar_auditoria(&con, &op, cj).unwrap();
    assert!(r.anomalias.iter().any(|a| a.regla_id == "TX01"),
        "Debe detectar TX01 con campo vacío");
}

#[test]
fn test_csv_limite_streaming() {
    // CSV grande simulado
    let mut contenido = "nombre,epp_asignado\n".to_string();
    for i in 0..1000 {
        contenido.push_str(&format!("Worker{},CASCO\n", i));
    }
    let f   = csv_temp(&contenido);
    let con = ConectorCsv::nuevo(f.path(), TS).con_limite(100);
    let r   = con.extraer().unwrap();
    assert_eq!(r.total, 100, "Con límite de 100, debe extraer exactamente 100");
}

#[test]
fn test_anomalias_en_orden_determinista() {
    let f   = csv_temp(CSV_COMPLETO);
    let con = ConectorCsv::nuevo(f.path(), TS);
    let cj1 = resolver_reglas("mineria", None);
    let cj2 = resolver_reglas("mineria", None);
    let r1  = ejecutar_auditoria(&con, &opts(f.path().to_str().unwrap()), cj1).unwrap();
    let r2  = ejecutar_auditoria(&con, &opts(f.path().to_str().unwrap()), cj2).unwrap();

    let ids1: Vec<&str> = r1.anomalias.iter().map(|a| a.regla_id.as_str()).collect();
    let ids2: Vec<&str> = r2.anomalias.iter().map(|a| a.regla_id.as_str()).collect();
    assert_eq!(ids1, ids2, "El orden de anomalías debe ser idéntico entre ejecuciones");
}

#[test]
fn test_hash_mide_64_chars_hex() {
    let f   = csv_temp("x\nval\n");
    let con = ConectorCsv::nuevo(f.path(), TS);
    let cj  = resolver_reglas("mineria", None);
    let r   = ejecutar_auditoria(&con, &opts(f.path().to_str().unwrap()), cj).unwrap();

    assert_eq!(r.hash_canonico.len(), 64);
    assert!(r.hash_canonico.chars().all(|c| c.is_ascii_hexdigit()),
        "hash_canonico debe ser hex válido");
}

#[test]
fn test_audit_id_es_determinista() {
    let f    = csv_temp("x\nval\n");
    let ruta = f.path().to_str().unwrap();
    let con  = ConectorCsv::nuevo(f.path(), TS);
    let cj1  = resolver_reglas("mineria", None);
    let cj2  = resolver_reglas("mineria", None);
    let r1   = ejecutar_auditoria(&con, &opts(ruta), cj1).unwrap();
    let r2   = ejecutar_auditoria(&con, &opts(ruta), cj2).unwrap();
    assert_eq!(r1.audit_id, r2.audit_id, "audit_id debe ser determinista");
    assert!(r1.audit_id.starts_with("audit-"), "audit_id debe empezar con 'audit-'");
}

#[test]
fn test_archivo_modificado_falla_verify() {
    let f    = csv_temp(CSV_COMPLETO);
    let ruta = f.path().to_str().unwrap();
    let con  = ConectorCsv::nuevo(f.path(), TS);
    let cj   = resolver_reglas("mineria", None);
    let r    = ejecutar_auditoria(&con, &opts(ruta), cj).unwrap();

    let dir  = tempfile::tempdir().unwrap();
    let pref = dir.path().join("rep").to_string_lossy().to_string();
    let (_pdf, json) = generar_evidencia(&r.evidencia, &r.hash_canonico, &pref).unwrap();

    // Modificar el JSON en disco
    let contenido = std::fs::read_to_string(&json).unwrap();
    let modificado = contenido.replace(&r.evidencia.audit_id, "ATACANTE-MOD");
    std::fs::write(&json, &modificado).unwrap();

    let ok = verificar_integridad(&json, &r.hash_canonico).unwrap();
    assert!(!ok, "Verificación DEBE FALLAR con archivo modificado");
}
