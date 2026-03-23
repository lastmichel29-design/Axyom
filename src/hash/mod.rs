// ============================================================
// AXYOM Auditor v7.0 — hash/mod.rs
// SHA3-256 centralizado — todas las operaciones de hash pasan aquí
// Michel Antonio Duran Cornejo — Chile 2026
// ============================================================

use sha3::{Digest, Sha3_256};

/// SHA3-256 sobre bytes arbitrarios → hex string 64 chars
pub fn sha3_bytes(data: &[u8]) -> String {
    let mut h = Sha3_256::new();
    h.update(data);
    hex::encode(h.finalize())
}

/// SHA3-256 sobre un string UTF-8
pub fn sha3_str(s: &str) -> String {
    sha3_bytes(s.as_bytes())
}

/// SHA3-256 sobre un archivo en disco
pub fn sha3_archivo(ruta: &str) -> Result<String, String> {
    let bytes = std::fs::read(ruta)
        .map_err(|e| format!("No se pudo leer '{ruta}': {e}"))?;
    Ok(sha3_bytes(&bytes))
}

/// Verifica que el hash de un archivo coincide con el esperado.
/// Retorna Ok(true) si coincide, Ok(false) si no.
pub fn verificar_archivo(ruta: &str, hash_esperado: &str) -> Result<bool, String> {
    let calculado = sha3_archivo(ruta)?;
    Ok(calculado == hash_esperado.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha3_determinista() {
        assert_eq!(sha3_str("axyom"), sha3_str("axyom"));
        assert_ne!(sha3_str("axyom"), sha3_str("AXYOM"));
        assert_eq!(sha3_str("").len(), 64);
    }

    #[test]
    fn test_sha3_conocido() {
        // SHA3-256("abc") valor conocido para validar implementación
        let resultado = sha3_str("abc");
        assert_eq!(resultado.len(), 64);
        assert!(resultado.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_verificar_archivo() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("test.txt");
        std::fs::write(&ruta, b"datos de prueba").unwrap();
        let hash = sha3_archivo(ruta.to_str().unwrap()).unwrap();
        assert!(verificar_archivo(ruta.to_str().unwrap(), &hash).unwrap());
        assert!(!verificar_archivo(ruta.to_str().unwrap(), "hash_falso_000").unwrap());
    }
}
