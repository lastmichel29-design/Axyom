// ============================================================
// AXYOM Auditor v7.0 — connectors/excel.rs
// CORRECCIÓN CALAMINE 0.24: worksheet_range() devuelve Result (no Option<Result>)
// → usar .map_err() en lugar de .ok_or_else()
// Michel Antonio Duran Cornejo — Chile 2026
// ============================================================

use std::collections::BTreeMap;
use std::path::PathBuf;

use calamine::{open_workbook_auto, Data, Reader};

use crate::connectors::{normalizar_campo, valor_limpio, ConectorReadonly};
use crate::types::{Entidad, ResultadoExtraccion, TipoFuente};

#[derive(Debug, Clone)]
pub struct ConectorExcel {
    pub ruta:      PathBuf,
    pub hoja:      Option<String>,
    pub timestamp: String,
}

impl ConectorExcel {
    pub fn nuevo(ruta: impl Into<PathBuf>, timestamp: impl Into<String>) -> Self {
        ConectorExcel { ruta: ruta.into(), hoja: None, timestamp: timestamp.into() }
    }

    pub fn con_hoja(mut self, hoja: impl Into<String>) -> Self {
        self.hoja = Some(hoja.into());
        self
    }
}

impl ConectorReadonly for ConectorExcel {
    fn extraer(&self) -> Result<ResultadoExtraccion, String> {
        let mut workbook = open_workbook_auto(&self.ruta)
            .map_err(|e| format!("No se pudo abrir Excel '{}': {e}", self.ruta.display()))?;

        let hoja = match &self.hoja {
            Some(h) => h.clone(),
            None    => workbook
                .sheet_names()
                .first()
                .cloned()
                .ok_or_else(|| "El archivo Excel no tiene hojas".to_string())?,
        };

        // CORRECCIÓN CALAMINE 0.24: worksheet_range() devuelve Result directamente
        // No se usa .ok_or_else() — se usa .map_err()
        let range = workbook
            .worksheet_range(&hoja)
            .map_err(|e| format!("Error leyendo hoja '{hoja}': {e}"))?;

        let mut rows = range.rows();

        // Primera fila = headers
        let headers: Vec<String> = rows
            .next()
            .ok_or_else(|| format!("La hoja '{hoja}' no tiene encabezados"))?
            .iter()
            .map(|c: &Data| normalizar_campo(&c.to_string()))
            .collect();

        let fuente_base = format!("EXCEL:{}", self.ruta.display());
        let mut entidades = Vec::new();

        for (idx, row) in rows.enumerate() {
            let mut campos = BTreeMap::new();
            for (i, cell) in row.iter().enumerate() {
                let key = headers.get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("col_{i}"));
                campos.insert(key, valor_limpio(&cell.to_string()));
            }
            entidades.push(Entidad {
                id:        format!("{}:fila_{}", fuente_base, idx + 2),
                fuente:    fuente_base.clone(),
                tipo:      TipoFuente::Excel,
                campos,
                timestamp: self.timestamp.clone(),
            });
        }

        Ok(ResultadoExtraccion {
            fuente: fuente_base,
            total:  entidades.len(),
            entidades,
        })
    }
}
