// ============================================================
// AXYOM Auditor v7.0 — rule_engine/yaml.rs
// Carga reglas desde archivo YAML externo
// Michel Antonio Duran Cornejo — Chile 2026
// ============================================================

use serde::Deserialize;
use std::path::Path;

use super::{Condicion, ConjuntoReglas, ReglaCondicional, ReglaSimple};
use crate::types::Severidad;

// ─── Estructuras YAML intermedias ─────────────────────────────

#[derive(Deserialize)]
struct YamlFile {
    meta:         YamlMeta,
    simples:      Vec<YamlSimple>,
    condicionales: Vec<YamlCondicional>,
}

#[derive(Deserialize)]
struct YamlMeta {
    conjunto: String,
    version:  String,
}

#[derive(Deserialize)]
struct YamlSimple {
    id:          String,
    descripcion: String,
    campo:       String,
    condicion:   String,
    valor:       Option<String>,
    normativa:   String,
    severidad:   String,
}

#[derive(Deserialize)]
struct YamlCondicional {
    id:                  String,
    descripcion:         String,
    campo_si:            String,
    condicion_si:        String,
    valor_si:            Option<String>,
    campo_entonces:      String,
    condicion_entonces:  String,
    valor_entonces:      Option<String>,
    normativa:           String,
    severidad:           String,
}

// ─── Función de carga ─────────────────────────────────────────

/// Carga un ConjuntoReglas desde un archivo YAML en disco.
/// Si la ruta no existe, devuelve error descriptivo.
pub fn cargar_yaml(ruta: &str) -> Result<ConjuntoReglas, String> {
    let contenido = std::fs::read_to_string(ruta)
        .map_err(|e| format!("No se pudo leer archivo de reglas '{}': {e}", ruta))?;

    cargar_desde_str(&contenido, ruta)
}

/// Carga desde un string YAML (útil en tests).
pub fn cargar_desde_str(yaml_str: &str, origen: &str) -> Result<ConjuntoReglas, String> {
    let archivo: YamlFile = serde_yaml::from_str(yaml_str)
        .map_err(|e| format!("YAML inválido en '{}': {e}", origen))?;

    let simples = archivo.simples.iter()
        .map(|r| {
            let condicion = Condicion::desde_str(&r.condicion, r.valor.as_deref())
                .map_err(|e| format!("Regla '{}': {e}", r.id))?;
            Ok(ReglaSimple {
                id:          r.id.clone(),
                descripcion: r.descripcion.clone(),
                campo:       r.campo.clone(),
                condicion,
                normativa:   r.normativa.clone(),
                severidad:   Severidad::desde_str(&r.severidad),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let condicionales = archivo.condicionales.iter()
        .map(|r| {
            let cond_si = Condicion::desde_str(&r.condicion_si, r.valor_si.as_deref())
                .map_err(|e| format!("Regla '{}' campo_si: {e}", r.id))?;
            let cond_ent = Condicion::desde_str(&r.condicion_entonces, r.valor_entonces.as_deref())
                .map_err(|e| format!("Regla '{}' campo_entonces: {e}", r.id))?;
            Ok(ReglaCondicional {
                id:                 r.id.clone(),
                descripcion:        r.descripcion.clone(),
                campo_si:           r.campo_si.clone(),
                condicion_si:       cond_si,
                campo_entonces:     r.campo_entonces.clone(),
                condicion_entonces: cond_ent,
                normativa:          r.normativa.clone(),
                severidad:          Severidad::desde_str(&r.severidad),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(ConjuntoReglas {
        nombre:  archivo.meta.conjunto,
        version: archivo.meta.version,
        simples,
        condicionales,
    })
}

/// Busca un archivo YAML de reglas en rutas estándar:
/// 1. ruta_custom si se proporciona
/// 2. rules/{nombre}.yaml (relativo al ejecutable)
/// 3. Fallback a reglas internas
pub fn resolver_reglas(nombre: &str, ruta_custom: Option<&str>) -> ConjuntoReglas {
    // 1. Ruta explícita
    if let Some(ruta) = ruta_custom {
        match cargar_yaml(ruta) {
            Ok(c)   => return c,
            Err(e)  => eprintln!("[WARN] No se pudo cargar '{}': {} — usando internas", ruta, e),
        }
    }

    // 2. rules/{nombre}.yaml relativo al ejecutable o al directorio actual
    let rutas_busqueda = [
        format!("rules/{}.yaml", nombre),
        format!("rules/{}.yml",  nombre),
    ];

    for ruta in &rutas_busqueda {
        if Path::new(ruta).exists() {
            match cargar_yaml(ruta) {
                Ok(c) => {
                    eprintln!("[INFO] Reglas cargadas desde '{}'", ruta);
                    return c;
                }
                Err(e) => eprintln!("[WARN] Error leyendo '{}': {} — usando internas", ruta, e),
            }
        }
    }

    // 3. Fallback a reglas internas compiladas
    eprintln!("[INFO] Usando reglas internas para '{}'", nombre);
    super::conjunto_interno(nombre)
}

#[cfg(test)]
mod tests {
    use super::*;

    const YAML_MINERIA: &str = r#"
meta:
  conjunto: "mineria_test"
  version:  "9.9.9"

simples:
  - id:          "T001"
    descripcion: "Campo X no vacío"
    campo:       "campo_x"
    condicion:   "no_vacio"
    normativa:   "Test"
    severidad:   "critico"

  - id:          "T002"
    descripcion: "Campo Y igual a VALOR"
    campo:       "campo_y"
    condicion:   "igual_a"
    valor:       "VALOR"
    normativa:   "Test"
    severidad:   "medio"

condicionales:
  - id:                  "TC001"
    descripcion:         "Si X → Y no vacío"
    campo_si:            "campo_x"
    condicion_si:        "no_vacio"
    campo_entonces:      "campo_y"
    condicion_entonces:  "no_vacio"
    normativa:           "Test"
    severidad:           "medio"
"#;

    #[test]
    fn test_cargar_yaml_basico() {
        let c = cargar_desde_str(YAML_MINERIA, "test").unwrap();
        assert_eq!(c.nombre,  "mineria_test");
        assert_eq!(c.version, "9.9.9");
        assert_eq!(c.simples.len(), 2);
        assert_eq!(c.condicionales.len(), 1);
    }

    #[test]
    fn test_cargar_yaml_condicion_igual_a() {
        let c = cargar_desde_str(YAML_MINERIA, "test").unwrap();
        let r = &c.simples[1];
        assert_eq!(r.id, "T002");
        assert_eq!(r.condicion, Condicion::IgualA("VALOR".to_string()));
    }

    #[test]
    fn test_yaml_invalido_da_error() {
        let r = cargar_desde_str("esto no es yaml válido: :::::", "test");
        assert!(r.is_err());
    }

    #[test]
    fn test_yaml_sin_valor_para_igual_a_da_error() {
        let yaml_malo = r#"
meta:
  conjunto: "test"
  version:  "1.0"
simples:
  - id: "X001"
    descripcion: "sin valor"
    campo: "x"
    condicion: "igual_a"
    normativa: "T"
    severidad: "medio"
condicionales: []
"#;
        let r = cargar_desde_str(yaml_malo, "test");
        assert!(r.is_err(), "igual_a sin valor debe dar error");
    }

    #[test]
    fn test_version_reglas_estable_desde_yaml() {
        let c = cargar_desde_str(YAML_MINERIA, "test").unwrap();
        let h1 = c.version_reglas().hash.clone();
        let h2 = c.version_reglas().hash.clone();
        assert_eq!(h1, h2);
    }
}
