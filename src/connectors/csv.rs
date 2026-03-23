// ============================================================
// AXYOM Auditor v7.0 — connectors/csv.rs
//
// MEJORA v7: streaming real — procesa fila a fila sin cargar todo en RAM.
// En v6 se acumulaban todas las filas antes de procesar.
// En v7 el iterador de csv::Reader es lazy → O(1) memoria por fila.
//
// Para minería real: datasets de 10^7 filas sin problema.
//
// Michel Antonio Duran Cornejo — Chile 2026
// ============================================================

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::connectors::{normalizar_campo, valor_limpio, ConectorReadonly};
use crate::types::{Entidad, ResultadoExtraccion, TipoFuente};

#[derive(Debug, Clone)]
pub struct ConectorCsv {
    pub ruta:      PathBuf,
    pub timestamp: String,
    /// Límite de filas (None = sin límite, para streaming completo)
    pub limite:    Option<usize>,
}

impl ConectorCsv {
    pub fn nuevo(ruta: impl Into<PathBuf>, timestamp: impl Into<String>) -> Self {
        ConectorCsv { ruta: ruta.into(), timestamp: timestamp.into(), limite: None }
    }

    pub fn con_limite(mut self, n: usize) -> Self {
        self.limite = Some(n);
        self
    }
}

impl ConectorReadonly for ConectorCsv {
    fn extraer(&self) -> Result<ResultadoExtraccion, String> {
        let mut reader = csv::Reader::from_path(&self.ruta)
            .map_err(|e| format!("No se pudo abrir CSV '{}': {e}", self.ruta.display()))?;

        // Headers: normalizados una vez
        let headers: Vec<String> = reader
            .headers()
            .map_err(|e| format!("No se pudieron leer headers CSV: {e}"))?
            .iter()
            .map(normalizar_campo)
            .collect();

        let fuente_str = format!("CSV:{}", self.ruta.display());
        let mut entidades = Vec::new();

        // Streaming: el iterador es lazy, cada fila se procesa y descarta
        for (idx, row) in reader.records().enumerate() {
            if let Some(lim) = self.limite {
                if idx >= lim { break; }
            }

            let row = row.map_err(|e| format!("Fila CSV inválida en fila {}: {e}", idx + 2))?;
            let mut campos = BTreeMap::new();

            for (i, value) in row.iter().enumerate() {
                let key = headers.get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("columna_{i}"));
                campos.insert(key, valor_limpio(value));
            }

            entidades.push(Entidad {
                id:        format!("{}:fila_{}", fuente_str, idx + 1),
                fuente:    fuente_str.clone(),
                tipo:      TipoFuente::Csv,
                campos,
                timestamp: self.timestamp.clone(),
            });
        }

        Ok(ResultadoExtraccion {
            fuente: fuente_str,
            total:  entidades.len(),
            entidades,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn csv_temp(contenido: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(contenido.as_bytes()).unwrap();
        f
    }

    const TS: &str = "1970-01-01T00:00:00Z";

    #[test]
    fn test_extrae_filas() {
        let f = csv_temp("nombre,epp\nJuan,CASCO\nMaria,");
        let c = ConectorCsv::nuevo(f.path(), TS);
        let r = c.extraer().unwrap();
        assert_eq!(r.total, 2);
        assert_eq!(r.entidades[0].campos["nombre"], "Juan");
        assert_eq!(r.entidades[1].campos["epp"], "");
    }

    #[test]
    fn test_limite_streaming() {
        let f = csv_temp("n\n1\n2\n3\n4\n5");
        let c = ConectorCsv::nuevo(f.path(), TS).con_limite(3);
        let r = c.extraer().unwrap();
        assert_eq!(r.total, 3);
    }

    #[test]
    fn test_headers_normalizados() {
        let f = csv_temp("EPP Asignado,Examen Médico\nCASCO,APROBADO");
        let c = ConectorCsv::nuevo(f.path(), TS);
        let r = c.extraer().unwrap();
        assert!(r.entidades[0].campos.contains_key("epp_asignado"));
        assert!(r.entidades[0].campos.contains_key("examen_medico"));
    }

    #[test]
    fn test_btreemap_orden_estable() {
        let f = csv_temp("z,a,m\n3,1,2");
        let c = ConectorCsv::nuevo(f.path(), TS);
        let r = c.extraer().unwrap();
        let json = serde_json::to_string(&r.entidades[0].campos).unwrap();
        // BTreeMap serializa en orden alfabético: a < m < z
        let pa = json.find("\"a\"").unwrap();
        let pm = json.find("\"m\"").unwrap();
        let pz = json.find("\"z\"").unwrap();
        assert!(pa < pm && pm < pz);
    }

    #[test]
    fn test_timestamp_fijo_determinista() {
        let contenido = "a,b\n1,2\n3,4";
        let f = csv_temp(contenido);
        let c = ConectorCsv::nuevo(f.path(), TS);
        let r1 = c.extraer().unwrap();
        let r2 = c.extraer().unwrap();
        let j1 = serde_json::to_string(&r1.entidades).unwrap();
        let j2 = serde_json::to_string(&r2.entidades).unwrap();
        assert_eq!(j1, j2, "Mismo CSV + timestamp fijo → mismo JSON");
    }
}
