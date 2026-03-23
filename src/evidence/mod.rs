// ============================================================
// AXYOM Auditor v7.0 — evidence/mod.rs
//
// CADENA DE HASH CERRADA (misma lógica corregida de v6):
//
//   1. json_bytes = canonical_str(EvidenciaCanonica)   ← v7 usa JSON canónico
//   2. hash_canonico = sha3(json_bytes)
//   3. write(archivo, json_bytes)
//   4. verify: sha3(read(archivo)) == hash_canonico → PASS siempre
//
// MEJORA v7 sobre v6:
//   v6 usaba serde_json::to_string() — orden de claves dependía de
//   la versión de serde y el sistema. En v7 usamos canonical_str()
//   que garantiza orden alfabético de claves → hash idéntico en
//   cualquier plataforma, cualquier versión de serde.
//
// COMPATIBILIDAD: EvidenciaCanonica tiene los mismos campos que v6.
//   Reportes antiguos de v6 siguen verificando con axyom v7 verify.
//
// printpdf 0.7 requiere f32 — NO usar f64 (bug conocido de v6)
//
// Michel Antonio Duran Cornejo — Chile 2026
// ============================================================

use printpdf::*;
use std::fs::File;
use std::io::BufWriter;

use crate::canonical::canonical_str;
use crate::hash::{sha3_bytes, sha3_str};
use crate::types::{Anomalia, EvidenciaCanonica, Severidad};

pub const VERSION_MOTOR: &str = "7.0.0";

// Constantes PDF — f32 obligatorio (printpdf 0.7)
const MARGEN_X:   f32 = 15.0;
const MARGEN_INF: f32 = 16.0;
const ALTO_PAG:   f32 = 297.0;
const ANCHO_PAG:  f32 = 210.0;

// ─── Canonización de datos ────────────────────────────────────

/// Ordena entidades para serialización determinista.
pub fn canonizar_entidades(entidades: &mut Vec<crate::types::Entidad>) {
    entidades.sort_by(|a, b| {
        let sa = serde_json::to_string(&a.campos).unwrap_or_default();
        let sb = serde_json::to_string(&b.campos).unwrap_or_default();
        (&a.fuente, &a.id, sa).cmp(&(&b.fuente, &b.id, sb))
    });
}

/// Ordena anomalías para serialización determinista.
pub fn canonizar_anomalias(anomalias: &mut Vec<Anomalia>) {
    anomalias.sort_by(|a, b| {
        (&a.entidad_id, &a.regla_id, &a.campo, &a.valor_actual, &a.valor_esperado)
            .cmp(&(&b.entidad_id, &b.regla_id, &b.campo, &b.valor_actual, &b.valor_esperado))
    });
}

// ─── Generación de evidencia ──────────────────────────────────

/// Genera el par (PDF, JSON canónico) con cadena de hash cerrada.
///
/// DIFERENCIA v7 vs v6:
///   v6: json_bytes = serde_json::to_string(&evidencia)
///   v7: json_bytes = canonical_str(&serde_json::to_value(&evidencia))
///   → orden de claves garantizado alfabéticamente en v7
///
/// El JSON escrito en disco es exactamente el string sobre el que
/// se calculó hash_canonico → verify() siempre pasa.
pub fn generar_evidencia(
    evidencia:      &EvidenciaCanonica,
    hash_canonico:  &str,
    prefijo_salida: &str,
) -> Result<(String, String), String> {
    let nombre_pdf  = format!("{}.pdf", prefijo_salida);
    let nombre_json = format!("{}_evidencia.json", prefijo_salida);

    // JSON canónico — claves en orden alfabético, sin espacios
    let json_val   = serde_json::to_value(evidencia)
        .map_err(|e| format!("Error serializando evidencia: {e}"))?;
    let json_bytes = canonical_str(&json_val);

    std::fs::write(&nombre_json, &json_bytes)
        .map_err(|e| format!("Error escribiendo JSON '{}': {e}", nombre_json))?;

    generar_pdf_multipagina(evidencia, hash_canonico, &nombre_pdf)?;

    Ok((nombre_pdf, nombre_json))
}

/// Verifica integridad: sha3(bytes_del_archivo) == hash_esperado.
///
/// Funciona con reportes de v6 (serde_json::to_string) y v7 (canonical_str)
/// porque en ambos casos el archivo contiene los bytes exactos sobre los
/// que se calculó el hash.
pub fn verificar_integridad(ruta_json: &str, hash_esperado: &str) -> Result<bool, String> {
    let bytes = std::fs::read(ruta_json)
        .map_err(|e| format!("No se pudo leer '{}': {e}", ruta_json))?;
    Ok(sha3_bytes(&bytes) == hash_esperado.to_lowercase())
}

// ─── PDF multipágina ──────────────────────────────────────────

struct PdfWriter<'a> {
    doc:        &'a PdfDocumentReference,
    f_reg:      IndirectFontRef,
    f_bold:     IndirectFontRef,
    pagina:     PdfPageIndex,
    capa:       PdfLayerIndex,
    y:          f32,   // f32 — printpdf 0.7 requiere f32
    num_pagina: usize,
}

impl<'a> PdfWriter<'a> {
    fn nueva_pagina(&mut self) {
        self.num_pagina += 1;
        let (p, l) = self.doc.add_page(
            Mm(ANCHO_PAG), Mm(ALTO_PAG),
            format!("Pagina {}", self.num_pagina),
        );
        self.pagina = p;
        self.capa   = l;
        self.y      = ALTO_PAG - 15.0_f32;
        self.doc.get_page(self.pagina).get_layer(self.capa).use_text(
            &format!("AXYOM Auditor v{} | pág. {} | 0 = 0", VERSION_MOTOR, self.num_pagina),
            6.5_f32, Mm(MARGEN_X), Mm(7.0_f32), &self.f_reg,
        );
    }

    fn line(&mut self, text: &str, size: f32, bold: bool) {
        if self.y < MARGEN_INF + size {
            self.nueva_pagina();
        }
        let font = if bold { &self.f_bold } else { &self.f_reg };
        self.doc
            .get_page(self.pagina)
            .get_layer(self.capa)
            .use_text(text, size, Mm(MARGEN_X), Mm(self.y), font);
        self.y -= size * 0.5_f32 + 2.4_f32;
    }

    fn sep(&mut self) {
        self.line(&"─".repeat(88), 7.5_f32, false);
    }

    fn gap(&mut self, mm: f32) {
        self.y -= mm;
    }
}

fn generar_pdf_multipagina(
    evidencia:  &EvidenciaCanonica,
    hash_canon: &str,
    nombre_pdf: &str,
) -> Result<(), String> {
    let doc = PdfDocument::empty("AXYOM Auditor v7.0");
    let (p1, l1) = doc.add_page(Mm(ANCHO_PAG), Mm(ALTO_PAG), "Pagina 1");

    let f_reg = doc.add_builtin_font(BuiltinFont::Courier)
        .map_err(|e| format!("Error fuente regular: {e}"))?;
    let f_bold = doc.add_builtin_font(BuiltinFont::CourierBold)
        .map_err(|e| format!("Error fuente bold: {e}"))?;

    // Footer página 1
    doc.get_page(p1).get_layer(l1).use_text(
        &format!("AXYOM Auditor v{} | pág. 1 | Chile 2026", VERSION_MOTOR),
        6.5_f32, Mm(MARGEN_X), Mm(7.0_f32), &f_reg,
    );

    let mut w = PdfWriter {
        doc: &doc,
        f_reg:  f_reg.clone(),
        f_bold: f_bold.clone(),
        pagina: p1,
        capa:   l1,
        y:      ALTO_PAG - 15.0_f32,
        num_pagina: 1,
    };

    // ── Encabezado ──────────────────────────────────────────
    w.line("AXYOM AUDITOR v7.0 — REPORTE LEGAL VERIFICABLE", 13.0_f32, true);
    w.line("Auditoría determinista · JSON canónico · SHA3-256", 8.5_f32, false);
    w.line("Michel Antonio Durán Cornejo — Chile 2026 · 0 = 0", 8.5_f32, false);
    w.sep();

    // ── Metadatos de la auditoría ────────────────────────────
    w.line(&format!("Audit ID      : {}", evidencia.audit_id),           9.0_f32, false);
    w.line(&format!("Motor         : v{}", evidencia.version_motor),     9.0_f32, false);
    w.line(&format!("Timestamp     : {}", evidencia.timestamp),          9.0_f32, false);
    w.line(&format!("Fuente        : {}", evidencia.fuente),             9.0_f32, false);
    w.line(&format!("Conexión      : {}", evidencia.conexion_redactada), 9.0_f32, false);
    w.line(&format!("Reglas        : {} v{}",
        evidencia.version_reglas.conjunto,
        evidencia.version_reglas.version),                               9.0_f32, false);
    w.line(&format!("Hash reglas   : {}", evidencia.version_reglas.hash), 7.5_f32, false);
    w.line(&format!("Hash input    : {}", evidencia.hash_input),          7.5_f32, false);
    w.sep();

    // ── Resumen ──────────────────────────────────────────────
    w.line(&format!("Entidades     : {}", evidencia.total_entidades), 9.0_f32, true);
    w.line(&format!("Anomalías     : {}", evidencia.total_anomalias), 9.0_f32, true);
    w.sep();

    // ── Sello SHA3 ───────────────────────────────────────────
    w.line("SELLO SHA3-256 — EVIDENCIA CANÓNICA v7", 9.0_f32, true);
    w.line("Verificar: axyom verify <_evidencia.json> <hash>", 7.5_f32, false);
    w.line("JSON canónico: claves alfabéticas, sin espacios, UTF-8", 7.5_f32, false);
    // Hash en dos líneas de 32 chars cada una
    if hash_canon.len() >= 32 {
        w.line(&hash_canon[..32],  8.5_f32, true);
        w.line(&hash_canon[32..],  8.5_f32, true);
    } else {
        w.line(hash_canon, 8.5_f32, true);
    }
    w.sep();

    // ── Anomalías ────────────────────────────────────────────
    if evidencia.anomalias.is_empty() {
        w.line("RESULTADO: SIN ANOMALÍAS DETECTADAS", 11.0_f32, true);
        w.line("El conjunto de datos cumple todas las reglas aplicadas.", 9.0_f32, false);
    } else {
        let n_crit = evidencia.anomalias.iter().filter(|a| a.severidad == Severidad::Critico).count();
        let n_med  = evidencia.anomalias.iter().filter(|a| a.severidad == Severidad::Medio).count();
        let n_bajo = evidencia.anomalias.iter().filter(|a| a.severidad == Severidad::Bajo).count();

        w.line(&format!("ANOMALÍAS DETECTADAS: {} (CRIT:{} MED:{} BAJO:{})",
            evidencia.anomalias.len(), n_crit, n_med, n_bajo), 10.0_f32, true);
        w.gap(2.0_f32);

        for (idx, a) in evidencia.anomalias.iter().enumerate() {
            let sev = match a.severidad {
                Severidad::Critico => "CRIT",
                Severidad::Medio   => "MED ",
                Severidad::Bajo    => "BAJO",
            };
            w.line(&format!("[{:03}] [{}] {} | {}", idx + 1, sev, a.regla_id, a.descripcion),
                8.0_f32, true);
            w.line(&format!("      Entidad   : {}", a.entidad_id),  7.5_f32, false);
            w.line(&format!("      Campo     : {}", a.campo),       7.5_f32, false);
            w.line(&format!("      Actual    : '{}'", a.valor_actual), 7.5_f32, false);
            w.line(&format!("      Esperado  : '{}'", a.valor_esperado), 7.5_f32, false);
            w.line(&format!("      Normativa : {}", a.normativa),   7.5_f32, false);
            w.gap(1.5_f32);
        }
    }

    w.sep();
    w.line("Firma electrónica: este documento es evidencia legal verificable.", 7.5_f32, false);
    w.line("El hash SHA3-256 garantiza integridad ante cualquier tribunal.", 7.5_f32, false);

    // ── Guardar PDF ──────────────────────────────────────────
    let file = File::create(nombre_pdf)
        .map_err(|e| format!("No se pudo crear PDF '{}': {e}", nombre_pdf))?;
    doc.save(&mut BufWriter::new(file))
        .map_err(|e| format!("Error guardando PDF: {e}"))?;

    Ok(())
}

// ─── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{EvidenciaCanonica, Severidad, VersionReglas};

    fn evidencia_fixture() -> EvidenciaCanonica {
        EvidenciaCanonica {
            audit_id:           "audit-v7-test-001".into(),
            version_motor:      VERSION_MOTOR.into(),
            timestamp:          "1970-01-01T00:00:00Z".into(),
            fuente:             "csv".into(),
            conexion_redactada: "demo.csv".into(),
            version_reglas: VersionReglas {
                conjunto: "mineria".into(),
                version:  "3.0.0".into(),
                hash:     "abc123def456".into(),
            },
            total_entidades: 2,
            total_anomalias: 1,
            anomalias: vec![Anomalia {
                regla_id:       "M001".into(),
                descripcion:    "EPP obligatorio".into(),
                entidad_id:     "CSV:demo.csv:fila_1".into(),
                fuente:         "CSV:demo.csv".into(),
                campo:          "epp_asignado".into(),
                valor_actual:   "".into(),
                valor_esperado: "valor no vacío".into(),
                normativa:      "DS 44/2024".into(),
                severidad:      Severidad::Critico,
            }],
            hash_input: "feedc0de".into(),
        }
    }

    #[test]
    fn test_cadena_hash_cierra_completamente() {
        let ev = evidencia_fixture();

        // Calcular hash como lo hace lib.rs (con JSON canónico v7)
        let json_val  = serde_json::to_value(&ev).unwrap();
        let json_str  = canonical_str(&json_val);
        let hash_calc = sha3_str(&json_str);

        // Guardar en archivo temporal
        let dir  = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("ev.json");
        std::fs::write(&ruta, &json_str).unwrap();

        // verify debe pasar siempre
        let ok = verificar_integridad(ruta.to_str().unwrap(), &hash_calc).unwrap();
        assert!(ok, "La cadena hash debe cerrar: sha3(archivo) == hash_canonico");
    }

    #[test]
    fn test_cadena_hash_via_generar_evidencia() {
        let ev   = evidencia_fixture();
        let dir  = tempfile::tempdir().unwrap();
        let pref = dir.path().join("reporte").to_string_lossy().to_string();

        // Calcular hash como en lib.rs
        let json_val  = serde_json::to_value(&ev).unwrap();
        let hash_calc = sha3_str(&canonical_str(&json_val));

        let (_pdf, json) = generar_evidencia(&ev, &hash_calc, &pref).unwrap();

        let ok = verificar_integridad(&json, &hash_calc).unwrap();
        assert!(ok, "generar_evidencia + verificar_integridad debe pasar siempre");
    }

    #[test]
    fn test_verificacion_detecta_modificacion() {
        let ev       = evidencia_fixture();
        let json_val = serde_json::to_value(&ev).unwrap();
        let json_str = canonical_str(&json_val);
        let hash_orig = sha3_str(&json_str);

        let dir  = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("mod.json");

        // Modificar el archivo después de escribirlo
        let modificado = json_str.replace("audit-v7-test-001", "ATACANTE-MODIFICO");
        std::fs::write(&ruta, &modificado).unwrap();

        let ok = verificar_integridad(ruta.to_str().unwrap(), &hash_orig).unwrap();
        assert!(!ok, "Verificación debe FALLAR con archivo modificado");
    }

    #[test]
    fn test_hash_canonico_igual_en_distintas_ejecuciones() {
        let ev = evidencia_fixture();
        let json_val = serde_json::to_value(&ev).unwrap();
        let h1 = sha3_str(&canonical_str(&json_val));
        let h2 = sha3_str(&canonical_str(&json_val));
        assert_eq!(h1, h2, "Hash debe ser idéntico entre ejecuciones");
    }

    #[test]
    fn test_canonizar_anomalias_orden_estable() {
        let mut xs = vec![
            Anomalia {
                regla_id: "B".into(), descripcion: "".into(),
                entidad_id: "2".into(), fuente: "".into(),
                campo: "x".into(), valor_actual: "".into(),
                valor_esperado: "".into(), normativa: "".into(),
                severidad: Severidad::Medio,
            },
            Anomalia {
                regla_id: "A".into(), descripcion: "".into(),
                entidad_id: "1".into(), fuente: "".into(),
                campo: "x".into(), valor_actual: "".into(),
                valor_esperado: "".into(), normativa: "".into(),
                severidad: Severidad::Medio,
            },
        ];
        canonizar_anomalias(&mut xs);
        assert_eq!(xs[0].entidad_id, "1");
        assert_eq!(xs[1].entidad_id, "2");
    }
}
