// ============================================================
// AXYOM Auditor v7.0 — connectors/mod.rs
// Idéntico a v6 — compatibilidad garantizada
// Michel Antonio Duran Cornejo — Chile 2026
// ============================================================

pub mod csv;
pub mod excel;
pub mod json_api;
pub mod sql;

use crate::types::{Entidad, ResultadoExtraccion};

/// Garantía técnica de readonly: no existe write(), insert(), update(), delete().
pub trait ConectorReadonly: Send + Sync {
    fn extraer(&self) -> Result<ResultadoExtraccion, String>;
}

/// Normaliza encabezados de forma determinista.
/// Letras: minúsculas. Acentos: eliminados. Espacios/especiales: underscore.
/// Underscores dobles o extremos: eliminados.
pub fn normalizar_campo(input: &str) -> String {
    let trimmed = input.trim();
    let mut out = String::with_capacity(trimmed.len());
    let mut last_underscore = false;

    for ch in trimmed.chars() {
        let mapped = match ch {
            'á'|'à'|'ä'|'â'|'Á'|'À'|'Ä'|'Â' => 'a',
            'é'|'è'|'ë'|'ê'|'É'|'È'|'Ë'|'Ê' => 'e',
            'í'|'ì'|'ï'|'î'|'Í'|'Ì'|'Ï'|'Î' => 'i',
            'ó'|'ò'|'ö'|'ô'|'Ó'|'Ò'|'Ö'|'Ô' => 'o',
            'ú'|'ù'|'ü'|'û'|'Ú'|'Ù'|'Ü'|'Û' => 'u',
            'ñ'|'Ñ'                            => 'n',
            c if c.is_ascii_alphanumeric()     => c.to_ascii_lowercase(),
            _                                  => '_',
        };

        if mapped == '_' {
            if !last_underscore { out.push('_'); }
            last_underscore = true;
        } else {
            out.push(mapped);
            last_underscore = false;
        }
    }

    out.trim_matches('_').to_string()
}

/// Limpia un valor de datos: trim + elimina BOM.
pub fn valor_limpio(s: &str) -> String {
    s.trim().replace('\u{feff}', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalizar_acentos() {
        assert_eq!(normalizar_campo("Capacitación Seguridad"), "capacitacion_seguridad");
        assert_eq!(normalizar_campo("Examen Médico"), "examen_medico");
        assert_eq!(normalizar_campo("ñoño"), "nono");
    }

    #[test]
    fn test_normalizar_sin_underscores_dobles() {
        assert_eq!(normalizar_campo("(campo)__doble"), "campo_doble");
        assert_eq!(normalizar_campo("__campo__"),      "campo");
    }

    #[test]
    fn test_valor_limpio_bom() {
        assert_eq!(valor_limpio("\u{feff}valor"), "valor");
        assert_eq!(valor_limpio("  dato  "),      "dato");
    }
}
