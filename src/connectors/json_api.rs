// ============================================================
// AXYOM Auditor v7.0 — connectors/json_api.rs
// Solo HTTP GET — nunca POST/PUT/DELETE
// Michel Antonio Duran Cornejo — Chile 2026
// ============================================================

use std::collections::BTreeMap;

use reqwest::blocking::Client;
use serde_json::Value;

use crate::connectors::{normalizar_campo, valor_limpio, ConectorReadonly};
use crate::types::{Entidad, ResultadoExtraccion, TipoFuente};

#[derive(Debug, Clone)]
pub struct ConectorAPI {
    pub url:           String,
    pub headers:       Vec<(String, String)>,
    pub nombre_fuente: String,
    pub timestamp:     String,
}

impl ConectorReadonly for ConectorAPI {
    fn extraer(&self) -> Result<ResultadoExtraccion, String> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Error creando cliente HTTP: {e}"))?;

        let mut req = client.get(&self.url);
        for (k, v) in &self.headers {
            req = req.header(k.as_str(), v.as_str());
        }

        let response = req.send()
            .map_err(|e| format!("Error GET '{}': {e}", self.url))?;

        let status = response.status();
        if !status.is_success() {
            return Err(format!("HTTP {} al consultar '{}'", status, self.url));
        }

        let json: Value = response.json()
            .map_err(|e| format!("Respuesta no es JSON válido: {e}"))?;

        let fuente = format!("API:{}", self.nombre_fuente);
        let entidades = json_a_entidades(&json, &fuente, &self.timestamp);
        let total = entidades.len();

        Ok(ResultadoExtraccion { fuente, total, entidades })
    }
}

fn json_a_entidades(json: &Value, fuente: &str, ts: &str) -> Vec<Entidad> {
    match json {
        Value::Array(arr) => arr.iter().enumerate()
            .map(|(i, item)| Entidad {
                id:        format!("{}:item_{}", fuente, i),
                fuente:    fuente.to_string(),
                tipo:      TipoFuente::Api,
                campos:    valor_a_campos(item),
                timestamp: ts.to_string(),
            })
            .collect(),
        _ => vec![Entidad {
            id:        format!("{}:root", fuente),
            fuente:    fuente.to_string(),
            tipo:      TipoFuente::Api,
            campos:    valor_a_campos(json),
            timestamp: ts.to_string(),
        }],
    }
}

fn valor_a_campos(val: &Value) -> BTreeMap<String, String> {
    let mut campos = BTreeMap::new();
    if let Value::Object(obj) = val {
        for (k, v) in obj {
            let nombre = normalizar_campo(k);
            let s = match v {
                Value::String(s) => valor_limpio(s),
                Value::Null      => "NULL".to_string(),
                Value::Bool(b)   => b.to_string(),
                Value::Number(n) => n.to_string(),
                other            => other.to_string(),
            };
            campos.insert(nombre, s);
        }
    }
    campos
}
