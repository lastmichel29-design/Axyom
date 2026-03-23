// ============================================================
// AXYOM Auditor v7.0 — canonical/mod.rs
//
// JSON CANÓNICO DETERMINISTA
//
// Problema que resuelve (detectado en análisis del txt):
//   En v6, dos ejecuciones con mismo input podían producir
//   JSON con distinto orden de claves → SHA3 distinto → bug legal.
//
// Solución v7:
//   Todas las claves de objetos JSON ordenadas alfabéticamente.
//   Arrays preservan su orden (orden de datos es semántico).
//   Encoding UTF-8 forzado sin BOM.
//
// Garantía matemática:
//   ∀ v1, v2 : v1 == v2 (como datos) ⟹ canonical(v1) == canonical(v2)
//
// Michel Antonio Duran Cornejo — Chile 2026
// ============================================================

use serde_json::Value;

/// Produce bytes JSON con claves de objetos en orden alfabético.
/// Arrays preservan su posición original (semántico).
pub fn canonical_json(value: &Value) -> Vec<u8> {
    canonical_value(value).into_bytes()
}

/// Produce string JSON canónico.
pub fn canonical_str(value: &Value) -> String {
    canonical_value(value)
}

fn canonical_value(value: &Value) -> String {
    match value {
        Value::Null         => "null".to_string(),
        Value::Bool(b)      => b.to_string(),
        Value::Number(n)    => n.to_string(),
        Value::String(s)    => serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string()),
        Value::Array(arr)   => {
            let items: Vec<String> = arr.iter().map(canonical_value).collect();
            format!("[{}]", items.join(","))
        }
        Value::Object(map) => {
            // Ordenar claves alfabéticamente — garantía de determinismo
            let mut pairs: Vec<(&String, &Value)> = map.iter().collect();
            pairs.sort_by_key(|(k, _)| k.as_str());

            let items: Vec<String> = pairs
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(k).unwrap_or_else(|_| "\"\"".to_string()),
                        canonical_value(v)
                    )
                })
                .collect();
            format!("{{{}}}", items.join(","))
        }
    }
}

/// Serializa cualquier tipo Serialize a JSON canónico.
pub fn serialize_canonical<T: serde::Serialize>(value: &T) -> Result<String, String> {
    let json_val = serde_json::to_value(value)
        .map_err(|e| format!("Error convirtiendo a Value: {e}"))?;
    Ok(canonical_str(&json_val))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_orden_claves_alfabetico() {
        let v = json!({ "z": 1, "a": 2, "m": 3 });
        let c = canonical_str(&v);
        // Debe aparecer: a, m, z
        let pos_a = c.find("\"a\"").unwrap();
        let pos_m = c.find("\"m\"").unwrap();
        let pos_z = c.find("\"z\"").unwrap();
        assert!(pos_a < pos_m, "a debe ir antes que m");
        assert!(pos_m < pos_z, "m debe ir antes que z");
    }

    #[test]
    fn test_determinismo_objeto() {
        // Dos objetos con mismo contenido pero distinto orden de inserción
        let v1 = json!({ "z": 1, "a": 2 });
        let v2 = json!({ "a": 2, "z": 1 });
        assert_eq!(canonical_str(&v1), canonical_str(&v2));
    }

    #[test]
    fn test_arrays_preservan_orden() {
        let v = json!([3, 1, 2]);
        let c = canonical_str(&v);
        assert_eq!(c, "[3,1,2]");
    }

    #[test]
    fn test_null_y_bool() {
        assert_eq!(canonical_str(&json!(null)),  "null");
        assert_eq!(canonical_str(&json!(true)),  "true");
        assert_eq!(canonical_str(&json!(false)), "false");
    }

    #[test]
    fn test_anidado() {
        let v = json!({ "b": { "z": 1, "a": 2 }, "a": [1,2,3] });
        let c = canonical_str(&v);
        // a del objeto raíz va antes que b
        assert!(c.find("\"a\":").unwrap() < c.find("\"b\":").unwrap());
        // Dentro del objeto anidado: a antes que z
        let b_start = c.find("\"b\":").unwrap();
        let inner = &c[b_start..];
        assert!(inner.find("\"a\"").unwrap() < inner.find("\"z\"").unwrap());
    }

    #[test]
    fn test_mismo_hash_distinto_orden_insercion() {
        use crate::hash::sha3_str;
        let v1 = json!({ "campo_b": "val", "campo_a": "otro" });
        let v2 = json!({ "campo_a": "otro", "campo_b": "val" });
        assert_eq!(sha3_str(&canonical_str(&v1)), sha3_str(&canonical_str(&v2)));
    }
}
