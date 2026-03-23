// ============================================================
// AXYOM Auditor v7.0 — connectors/sql.rs
// SQLite readonly real — sin journal_mode forzado (problemático con archivos RO)
// Michel Antonio Duran Cornejo — Chile 2026
// ============================================================

use std::collections::BTreeMap;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use sqlx::{Column, Row, TypeInfo};
use tokio::runtime::Runtime;

use crate::connectors::{normalizar_campo, valor_limpio, ConectorReadonly};
use crate::types::{Entidad, ResultadoExtraccion, TipoFuente};

#[derive(Debug, Clone)]
pub struct ConectorSQL {
    pub connection_string: String,
    pub query:             String,
    pub nombre_fuente:     String,
    pub timestamp:         String,
}

impl ConectorReadonly for ConectorSQL {
    fn extraer(&self) -> Result<ResultadoExtraccion, String> {
        validar_query_readonly(&self.query)?;

        let rt = Runtime::new()
            .map_err(|e| format!("Error creando runtime tokio: {e}"))?;

        let entidades = rt.block_on(async {
            let conn_str = asegurar_readonly(&self.connection_string);

            let opts = conn_str
                .parse::<SqliteConnectOptions>()
                .map_err(|e| format!("Connection string inválida: {e}"))?
                .read_only(true);

            let pool = SqlitePool::connect_with(opts).await
                .map_err(|e| format!("No se pudo conectar a la BD: {e}"))?;

            let rows = sqlx::query(&self.query)
                .fetch_all(&pool)
                .await
                .map_err(|e| format!("Error ejecutando query: {e}"))?;

            let fuente = format!("SQL:{}", self.nombre_fuente);
            let mut entidades: Vec<Entidad> = Vec::new();

            for (idx, row) in rows.iter().enumerate() {
                let mut campos = BTreeMap::new();
                for col in row.columns() {
                    let nombre = normalizar_campo(col.name());
                    let valor  = extraer_valor_tipado(row, col.ordinal(), col.type_info().name());
                    campos.insert(nombre, valor);
                }
                entidades.push(Entidad {
                    id:        format!("{}:row_{}", fuente, idx),
                    fuente:    fuente.clone(),
                    tipo:      TipoFuente::Sql,
                    campos,
                    timestamp: self.timestamp.clone(),
                });
            }

            Ok::<Vec<Entidad>, String>(entidades)
        })?;

        let total = entidades.len();
        Ok(ResultadoExtraccion {
            fuente: format!("SQL:{}", self.nombre_fuente),
            total,
            entidades,
        })
    }
}

fn extraer_valor_tipado(
    row:       &sqlx::sqlite::SqliteRow,
    ordinal:   usize,
    type_name: &str,
) -> String {
    match type_name.to_uppercase().as_str() {
        "INTEGER" | "INT" | "BIGINT" | "SMALLINT" | "TINYINT" => {
            row.try_get::<i64, _>(ordinal)
                .map(|v| v.to_string())
                .or_else(|_| row.try_get::<i32, _>(ordinal).map(|v| v.to_string()))
                .unwrap_or_else(|_| "NULL".to_string())
        }
        "REAL" | "FLOAT" | "DOUBLE" => {
            row.try_get::<f64, _>(ordinal)
                .map(|v| v.to_string())
                .unwrap_or_else(|_| "NULL".to_string())
        }
        "NUMERIC" | "DECIMAL" => {
            row.try_get::<String, _>(ordinal)
                .unwrap_or_else(|_| "NULL".to_string())
        }
        "BOOLEAN" | "BOOL" => {
            row.try_get::<bool, _>(ordinal)
                .map(|v| if v { "true" } else { "false" }.to_string())
                .unwrap_or_else(|_| "NULL".to_string())
        }
        "BLOB" => "[BLOB]".to_string(),
        "NULL"  => "NULL".to_string(),
        _ => {
            row.try_get::<String, _>(ordinal)
                .map(|v| valor_limpio(&v))
                .or_else(|_| row.try_get::<i64, _>(ordinal).map(|v| v.to_string()))
                .or_else(|_| row.try_get::<f64, _>(ordinal).map(|v| v.to_string()))
                .unwrap_or_else(|_| "NULL".to_string())
        }
    }
}

fn validar_query_readonly(query: &str) -> Result<(), String> {
    let sin_comentarios = query
        .lines()
        .map(|line| line.find("--").map_or(line, |pos| &line[..pos]))
        .collect::<Vec<_>>()
        .join(" ");

    let q = sin_comentarios.trim().to_lowercase();

    if !q.starts_with("select") && !q.starts_with("with") && !q.starts_with("pragma table_info") {
        return Err(format!(
            "AXYOM: solo SELECT permitido. Query empieza con: '{}'",
            &query.trim()[..query.trim().len().min(40)]
        ));
    }

    let prohibidas = [
        "insert ", "update ",  "delete ", "drop ",
        "alter ",  "truncate ", "attach ", "detach ",
        "create ", "replace ", "vacuum",   "reindex",
        "pragma journal_mode", "pragma wal", "pragma locking_mode",
    ];

    for kw in &prohibidas {
        if q.contains(kw) {
            return Err(format!(
                "AXYOM: query contiene operación prohibida '{}'. Solo SELECT.",
                kw.trim()
            ));
        }
    }

    Ok(())
}

fn asegurar_readonly(conn: &str) -> String {
    if conn.contains("mode=ro") { conn.to_string() }
    else if conn.contains('?')  { format!("{}&mode=ro", conn) }
    else                        { format!("{}?mode=ro", conn) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_valido() {
        assert!(validar_query_readonly("SELECT * FROM t").is_ok());
        assert!(validar_query_readonly("WITH cte AS (SELECT 1) SELECT * FROM cte").is_ok());
        assert!(validar_query_readonly("pragma table_info(trabajadores)").is_ok());
    }

    #[test]
    fn test_bloqueados() {
        assert!(validar_query_readonly("INSERT INTO t VALUES (1)").is_err());
        assert!(validar_query_readonly("DROP TABLE t").is_err());
        assert!(validar_query_readonly("PRAGMA journal_mode=WAL").is_err());
    }

    #[test]
    fn test_readonly_url() {
        assert_eq!(asegurar_readonly("sqlite:datos.db"), "sqlite:datos.db?mode=ro");
        assert_eq!(asegurar_readonly("sqlite:datos.db?mode=ro"), "sqlite:datos.db?mode=ro");
    }
}
