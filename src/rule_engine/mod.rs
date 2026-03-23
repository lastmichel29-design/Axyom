// ============================================================
// AXYOM Auditor v7.0 — rule_engine/mod.rs
//
// MEJORA v7: reglas cargadas desde YAML externo.
// Cambiar normativa = editar el YAML, sin recompilar.
// Backward compat: fallback a reglas internas si no hay YAML.
//
// Michel Antonio Duran Cornejo — Chile 2026
// ============================================================

pub mod yaml;

use crate::hash::sha3_str;
use crate::types::{Anomalia, Entidad, Severidad, VersionReglas};
use serde::{Deserialize, Serialize};

// ─── Estructuras de reglas ────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Condicion {
    NoVacio,
    IgualA(String),   // case-insensitive en evaluación
}

impl Condicion {
    pub fn desde_str(c: &str, valor: Option<&str>) -> Result<Self, String> {
        match c.to_lowercase().as_str() {
            "no_vacio" | "not_empty" => Ok(Condicion::NoVacio),
            "igual_a"  | "equal_to"  => Ok(Condicion::IgualA(
                valor.ok_or("Condición igual_a requiere campo 'valor'")?.to_uppercase()
            )),
            otro => Err(format!("Condición desconocida: '{otro}'")),
        }
    }

    pub fn evaluar(&self, valor: &str) -> (bool, String) {
        match self {
            Condicion::NoVacio => (
                valor.trim().is_empty(),
                "valor no vacío".to_string(),
            ),
            Condicion::IgualA(esperado) => (
                valor.trim().to_uppercase() != *esperado,
                format!("igual a '{}'", esperado),
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReglaSimple {
    pub id:          String,
    pub descripcion: String,
    pub campo:       String,
    pub condicion:   Condicion,
    pub normativa:   String,
    pub severidad:   Severidad,
}

#[derive(Debug, Clone)]
pub struct ReglaCondicional {
    pub id:                 String,
    pub descripcion:        String,
    pub campo_si:           String,
    pub condicion_si:       Condicion,
    pub campo_entonces:     String,
    pub condicion_entonces: Condicion,
    pub normativa:          String,
    pub severidad:          Severidad,
}

#[derive(Debug, Clone)]
pub struct ConjuntoReglas {
    pub nombre:        String,
    pub version:       String,
    pub simples:       Vec<ReglaSimple>,
    pub condicionales: Vec<ReglaCondicional>,
}

impl ConjuntoReglas {
    pub fn version_reglas(&self) -> VersionReglas {
        let ids_simples: Vec<&str>  = self.simples.iter().map(|r| r.id.as_str()).collect();
        let ids_cond:    Vec<&str>  = self.condicionales.iter().map(|r| r.id.as_str()).collect();
        let basis = serde_json::json!({
            "nombre":        self.nombre,
            "version":       self.version,
            "simples":       ids_simples,
            "condicionales": ids_cond,
        });
        VersionReglas {
            conjunto: self.nombre.clone(),
            version:  self.version.clone(),
            hash:     sha3_str(&serde_json::to_string(&basis).unwrap()),
        }
    }
}

// ─── Motor de evaluación ──────────────────────────────────────

pub struct MotorReglas {
    pub conjunto: ConjuntoReglas,
}

impl MotorReglas {
    pub fn nuevo(conjunto: ConjuntoReglas) -> Self {
        Self { conjunto }
    }

    /// Evalúa todas las reglas sobre todas las entidades.
    /// Resultado en orden determinista (sort por entidad_id + regla_id).
    pub fn evaluar(&self, entidades: &[Entidad]) -> Vec<Anomalia> {
        let mut out = Vec::new();

        for entidad in entidades {
            for regla in &self.conjunto.simples {
                if let Some(a) = evaluar_simple(regla, entidad) {
                    out.push(a);
                }
            }
            for regla in &self.conjunto.condicionales {
                if let Some(a) = evaluar_condicional(regla, entidad) {
                    out.push(a);
                }
            }
        }

        // Orden determinista
        out.sort_by(|a, b| {
            (&a.entidad_id, &a.regla_id, &a.campo, &a.valor_actual)
                .cmp(&(&b.entidad_id, &b.regla_id, &b.campo, &b.valor_actual))
        });

        out
    }
}

fn valor_de(entidad: &Entidad, campo: &str) -> String {
    entidad.campos.get(campo).cloned().unwrap_or_default()
}

fn evaluar_simple(regla: &ReglaSimple, entidad: &Entidad) -> Option<Anomalia> {
    let actual = valor_de(entidad, &regla.campo);
    let (fallo, esperado) = regla.condicion.evaluar(&actual);
    if !fallo { return None; }
    Some(Anomalia {
        regla_id:       regla.id.clone(),
        descripcion:    regla.descripcion.clone(),
        entidad_id:     entidad.id.clone(),
        fuente:         entidad.fuente.clone(),
        campo:          regla.campo.clone(),
        valor_actual:   actual,
        valor_esperado: esperado,
        normativa:      regla.normativa.clone(),
        severidad:      regla.severidad,
    })
}

fn evaluar_condicional(regla: &ReglaCondicional, entidad: &Entidad) -> Option<Anomalia> {
    let valor_si = valor_de(entidad, &regla.campo_si);
    let (no_cumple_si, _) = regla.condicion_si.evaluar(&valor_si);
    if no_cumple_si { return None; }  // condición SI no se activa

    let valor_then = valor_de(entidad, &regla.campo_entonces);
    let (fallo, esperado) = regla.condicion_entonces.evaluar(&valor_then);
    if !fallo { return None; }

    Some(Anomalia {
        regla_id:       regla.id.clone(),
        descripcion:    regla.descripcion.clone(),
        entidad_id:     entidad.id.clone(),
        fuente:         entidad.fuente.clone(),
        campo:          format!("{}→{}", regla.campo_si, regla.campo_entonces),
        valor_actual:   valor_then,
        valor_esperado: esperado,
        normativa:      regla.normativa.clone(),
        severidad:      regla.severidad,
    })
}

// ─── Reglas internas (fallback si no hay YAML) ───────────────

pub fn conjunto_interno(nombre: &str) -> ConjuntoReglas {
    match nombre.to_lowercase().as_str() {
        "banca" => reglas_banca(),
        "legal"  => reglas_legal(),
        _        => reglas_mineria(),
    }
}

fn r_simple(id: &str, desc: &str, campo: &str, cond: Condicion, norm: &str, sev: Severidad) -> ReglaSimple {
    ReglaSimple {
        id:          id.to_string(),
        descripcion: desc.to_string(),
        campo:       campo.to_string(),
        condicion:   cond,
        normativa:   norm.to_string(),
        severidad:   sev,
    }
}

fn reglas_mineria() -> ConjuntoReglas {
    ConjuntoReglas {
        nombre:  "mineria".to_string(),
        version: "3.0.0".to_string(),
        simples: vec![
            r_simple("M001", "EPP asignado obligatorio",          "epp_asignado",          Condicion::NoVacio,             "DS 44/2024 art. 37", Severidad::Critico),
            r_simple("M002", "Fecha de contrato obligatoria",     "fecha_contrato",         Condicion::NoVacio,             "Ley 16.744",         Severidad::Medio),
            r_simple("M003", "Examen médico debe ser APROBADO",   "examen_medico",          Condicion::IgualA("APROBADO".into()), "DS 594",        Severidad::Critico),
            r_simple("M004", "Capacitación de seguridad obligatoria", "capacitacion_seguridad", Condicion::NoVacio,         "DS 40",              Severidad::Medio),
        ],
        condicionales: vec![
            ReglaCondicional {
                id: "MC001".into(), descripcion: "Si examen APROBADO → capacitación no puede faltar".into(),
                campo_si: "examen_medico".into(), condicion_si: Condicion::IgualA("APROBADO".into()),
                campo_entonces: "capacitacion_seguridad".into(), condicion_entonces: Condicion::NoVacio,
                normativa: "DS 40".into(), severidad: Severidad::Medio,
            },
            ReglaCondicional {
                id: "MC002".into(), descripcion: "Si existe contrato → EPP debe estar asignado".into(),
                campo_si: "fecha_contrato".into(), condicion_si: Condicion::NoVacio,
                campo_entonces: "epp_asignado".into(), condicion_entonces: Condicion::NoVacio,
                normativa: "DS 44/2024 art. 37".into(), severidad: Severidad::Critico,
            },
        ],
    }
}

fn reglas_banca() -> ConjuntoReglas {
    ConjuntoReglas {
        nombre:  "banca".to_string(),
        version: "3.0.0".to_string(),
        simples: vec![
            r_simple("B001", "Cliente debe tener RUT",       "rut",          Condicion::NoVacio, "CMF Circular 3.507", Severidad::Critico),
            r_simple("B002", "Cuenta debe tener estado",     "estado_cuenta", Condicion::NoVacio, "CMF Circular 3.507", Severidad::Medio),
        ],
        condicionales: vec![],
    }
}

fn reglas_legal() -> ConjuntoReglas {
    ConjuntoReglas {
        nombre:  "legal".to_string(),
        version: "3.0.0".to_string(),
        simples: vec![
            r_simple("L001", "Expediente debe tener rol",   "rol",   Condicion::NoVacio, "Ley 19.880", Severidad::Critico),
            r_simple("L002", "Expediente debe tener fecha", "fecha", Condicion::NoVacio, "Ley 19.880", Severidad::Medio),
        ],
        condicionales: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use crate::types::TipoFuente;

    fn entidad(campos_vec: Vec<(&str, &str)>) -> Entidad {
        let mut campos = BTreeMap::new();
        for (k, v) in campos_vec { campos.insert(k.to_string(), v.to_string()); }
        Entidad {
            id:        "test:fila_1".to_string(),
            fuente:    "CSV:test.csv".to_string(),
            tipo:      TipoFuente::Csv,
            campos,
            timestamp: "1970-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_condicion_igual_a_case_insensitive() {
        let c = Condicion::IgualA("APROBADO".to_string());
        assert!(!c.evaluar("aprobado").0);
        assert!(!c.evaluar("APROBADO").0);
        assert!( c.evaluar("PENDIENTE").0);
    }

    #[test]
    fn test_condicion_no_vacio() {
        let c = Condicion::NoVacio;
        assert!( c.evaluar("").0);
        assert!( c.evaluar("  ").0);
        assert!(!c.evaluar("dato").0);
    }

    #[test]
    fn test_motor_mineria_juan_sin_anomalias() {
        let motor = MotorReglas::nuevo(reglas_mineria());
        let juan = entidad(vec![
            ("epp_asignado",          "CASCO"),
            ("fecha_contrato",        "2024-01-15"),
            ("examen_medico",         "APROBADO"),
            ("capacitacion_seguridad","COMPLETADA"),
        ]);
        assert!(motor.evaluar(&[juan]).is_empty());
    }

    #[test]
    fn test_motor_mineria_maria_con_anomalias() {
        let motor = MotorReglas::nuevo(reglas_mineria());
        let maria = entidad(vec![
            ("epp_asignado",          ""),
            ("fecha_contrato",        "2024-02-01"),
            ("examen_medico",         "APROBADO"),
            ("capacitacion_seguridad","COMPLETADA"),
        ]);
        let anomalias = motor.evaluar(&[maria]);
        let ids: Vec<&str> = anomalias.iter().map(|a| a.regla_id.as_str()).collect();
        assert!(ids.contains(&"M001"), "debe detectar M001");
        assert!(ids.contains(&"MC002"), "debe detectar MC002");
    }

    #[test]
    fn test_motor_orden_determinista() {
        let motor = MotorReglas::nuevo(reglas_mineria());
        let e = entidad(vec![
            ("epp_asignado", ""), ("fecha_contrato", ""),
            ("examen_medico", "PENDIENTE"), ("capacitacion_seguridad", ""),
        ]);
        let r1 = motor.evaluar(&[e.clone()]);
        let r2 = motor.evaluar(&[e]);
        assert_eq!(
            serde_json::to_string(&r1).unwrap(),
            serde_json::to_string(&r2).unwrap()
        );
    }
}

// ─── API pública para pipeline (paralelismo rayon) ────────────

/// Wrapper público de evaluar_simple — necesario para par_iter() en pipeline.rs
pub fn evaluar_simple_pub(regla: &ReglaSimple, entidad: &crate::types::Entidad) -> Option<crate::types::Anomalia> {
    evaluar_simple(regla, entidad)
}

/// Wrapper público de evaluar_condicional — necesario para par_iter() en pipeline.rs
pub fn evaluar_condicional_pub(regla: &ReglaCondicional, entidad: &crate::types::Entidad) -> Option<crate::types::Anomalia> {
    evaluar_condicional(regla, entidad)
}
