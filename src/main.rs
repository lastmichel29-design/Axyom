// ============================================================
// AXYOM Auditor v7.0 — main.rs
//
// CLI compatible con v6:
//   axyom audit --fuente csv --conexion datos.csv --reglas mineria --export
//   axyom verify reporte_evidencia.json <hash>
//   axyom info
//
// Nuevos en v7:
//   axyom audit --ruta-reglas rules/mineria.yaml  (YAML externo)
//   axyom api [--puerto 8080]                     (API REST)
//
// Michel Antonio Durán Cornejo — Chile 2026
// ============================================================

use clap::{Parser, Subcommand};
use colored::Colorize;

use axyom::connectors::csv::ConectorCsv;
use axyom::connectors::excel::ConectorExcel;
use axyom::connectors::json_api::ConectorAPI;
use axyom::connectors::sql::ConectorSQL;
use axyom::evidence::{generar_evidencia, verificar_integridad, VERSION_MOTOR};
use axyom::rule_engine::yaml::resolver_reglas;
use axyom::types::{AuditOptions, Severidad};
use axyom::ejecutar_auditoria;

// ─── Definición CLI ───────────────────────────────────────────

#[derive(Parser)]
#[command(
    name    = "axyom",
    version = VERSION_MOTOR,
    about   = "AXYOM Auditor v7.0 — auditoría determinista y verificable\n\
               Reglas YAML · JSON canónico · SHA3-256 · API REST\n\
               Axioma: 0 = 0 | Michel Antonio Durán Cornejo — Chile 2026"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Auditar una fuente de datos (solo lectura garantizada)
    Audit {
        #[arg(long, help = "csv | excel | sql | api")]
        fuente: String,

        #[arg(long, help = "Ruta al archivo, connection string o URL")]
        conexion: String,

        #[arg(long, default_value = "mineria", help = "mineria | banca | legal")]
        reglas: String,

        #[arg(long, help = "Ruta a archivo YAML de reglas personalizado")]
        ruta_reglas: Option<String>,

        #[arg(long, help = "Timestamp RFC3339 fijo (omitir → 1970-01-01T00:00:00Z)")]
        timestamp: Option<String>,

        #[arg(short, long, help = "Exportar PDF + JSON canónico verificable")]
        export: bool,

        #[arg(long, default_value = "reporte", help = "Prefijo para archivos de salida")]
        salida: String,

        #[arg(long, help = "[SQL] Query SELECT a ejecutar")]
        query: Option<String>,

        #[arg(long, help = "[Excel] Nombre de hoja específica")]
        hoja: Option<String>,
    },

    /// Verificar integridad de un reporte exportado
    Verify {
        #[arg(help = "Ruta al archivo JSON de evidencia")]
        json: String,

        #[arg(help = "Hash SHA3-256 canónico del reporte")]
        hash: String,
    },

    /// Información del sistema AXYOM
    Info,

    /// Iniciar servidor API REST local (127.0.0.1)
    Api {
        #[arg(long, default_value = "8080", help = "Puerto de escucha")]
        puerto: u16,
    },
}

// ─── Entry point ─────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Info  => cmd_info(),
        Cmd::Verify { json, hash } => cmd_verify(&json, &hash),
        Cmd::Api { puerto } => cmd_api(puerto),
        Cmd::Audit {
            fuente, conexion, reglas, ruta_reglas,
            timestamp, export, salida, query, hoja
        } => cmd_audit(
            &fuente, &conexion, &reglas, ruta_reglas.as_deref(),
            timestamp, export, &salida, query, hoja
        ),
    }
}

// ─── Comando: info ────────────────────────────────────────────

fn cmd_info() {
    let sep = "═".repeat(70);
    println!("{}", sep.cyan());
    println!("{}", format!("  AXYOM Auditor v{}", VERSION_MOTOR).cyan().bold());
    println!("{}", "  Michel Antonio Durán Cornejo — Chile 2026".cyan());
    println!("{}", "  Axioma: 0 = 0  |  \"Auditoría formal verificable\"".cyan());
    println!("{}", sep.cyan());
    println!();
    println!("  Motor         : Rust 1.85+ (rayon paralelo)");
    println!("  Conectores    : CSV streaming, Excel, SQL SQLite, API JSON");
    println!("  Reglas        : YAML externo (rules/*.yaml) o internas");
    println!("  Hash          : SHA3-256, JSON canónico, reproducible");
    println!("  Evidencia     : PDF multipágina + JSON verificable");
    println!("  API REST      : axyom api [--puerto 8080]");
    println!();
    println!("  Cadena de custodia:");
    println!("    canonical_str(EvidenciaCanonica) → sha3 → hash_canonico");
    println!("    sha3(bytes_archivo_evidencia.json) == hash_canonico → PASS");
    println!();
    println!("  Uso básico:");
    println!("    axyom audit --fuente csv --conexion datos.csv --reglas mineria --export");
    println!("    axyom audit --fuente csv --conexion datos.csv --ruta-reglas rules/mineria.yaml");
    println!("    axyom verify reporte_evidencia.json <hash>");
    println!("    axyom api --puerto 8080");
    println!();
    println!("  Reglas YAML disponibles:");

    for nombre in ["mineria", "banca", "legal"] {
        let rutas = [format!("rules/{}.yaml", nombre), format!("rules/{}.yml", nombre)];
        let encontrado = rutas.iter().any(|r| std::path::Path::new(r).exists());
        let estado = if encontrado {
            format!("✓ {}", rutas[0]).green().to_string()
        } else {
            format!("· {nombre} (interna)", ).dimmed().to_string()
        };
        println!("    {}", estado);
    }
    println!();
    println!("{}", sep.cyan());
    println!("  AXYOM v{} | 128 teoremas Coq | 0 admits core", VERSION_MOTOR);
    println!("{}", sep.cyan());
}

// ─── Comando: verify ─────────────────────────────────────────

fn cmd_verify(json: &str, hash: &str) {
    match verificar_integridad(json, hash) {
        Ok(true) => {
            println!("{} Integridad verificada", "PASS".green().bold());
            println!("  Archivo : {}", json);
            println!("  Hash    : {}", hash);
        }
        Ok(false) => {
            println!("{} Hash no coincide — archivo modificado o hash incorrecto",
                "FAIL".red().bold());
            println!("  Archivo : {}", json);
            std::process::exit(2);
        }
        Err(e) => {
            eprintln!("{} {}", "ERROR".red().bold(), e);
            std::process::exit(1);
        }
    }
}

// ─── Comando: api ────────────────────────────────────────────

fn cmd_api(puerto: u16) {
    println!("{}", "═".repeat(70).cyan());
    println!("{}", format!("  AXYOM API REST v{}", VERSION_MOTOR).cyan().bold());
    println!("{}", format!("  http://127.0.0.1:{}", puerto).cyan());
    println!("{}", "═".repeat(70).cyan());
    println!();
    println!("  Endpoints:");
    println!("    GET  /health  — estado del servidor");
    println!("    GET  /version — versión del motor");
    println!("    POST /audit   — auditar CSV");
    println!();
    println!("  Ejemplo:");
    println!("    curl -X POST http://127.0.0.1:{}/audit \\", puerto);
    println!("      -H 'Content-Type: application/json' \\");
    println!("      -d '{{\"csv\": \"demo_trabajadores.csv\", \"reglas\": \"mineria\"}}'");
    println!();

    let rt = tokio::runtime::Runtime::new().expect("Error creando runtime tokio");
    if let Err(e) = rt.block_on(axyom::api::start(puerto)) {
        eprintln!("{} {}", "ERROR API".red().bold(), e);
        std::process::exit(1);
    }
}

// ─── Comando: audit ──────────────────────────────────────────

fn cmd_audit(
    fuente:      &str,
    conexion:    &str,
    reglas:      &str,
    ruta_reglas: Option<&str>,
    timestamp:   Option<String>,
    export:      bool,
    salida:      &str,
    query:       Option<String>,
    hoja:        Option<String>,
) {
    let ts = timestamp.clone()
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());

    // Resolver conjunto de reglas (YAML externo → fallback interno)
    let conjunto = resolver_reglas(reglas, ruta_reglas);

    let options = AuditOptions {
        fuente:      fuente.to_string(),
        conexion:    conexion.to_string(),
        reglas:      reglas.to_string(),
        timestamp:   timestamp,
        ruta_reglas: ruta_reglas.map(|s| s.to_string()),
    };

    // Seleccionar conector según fuente
    let resultado = match fuente.to_lowercase().as_str() {
        "csv" => ejecutar_auditoria(
            &ConectorCsv::nuevo(conexion, &ts),
            &options,
            conjunto,
        ),
        "excel" | "xlsx" => {
            let mut c = ConectorExcel::nuevo(conexion, &ts);
            if let Some(h) = hoja { c = c.con_hoja(h); }
            ejecutar_auditoria(&c, &options, conjunto)
        },
        "sql" | "sqlite" => ejecutar_auditoria(
            &ConectorSQL {
                connection_string: conexion.to_string(),
                query:             query.unwrap_or_else(|| "SELECT * FROM datos LIMIT 1000".to_string()),
                nombre_fuente:     "SQL".to_string(),
                timestamp:         ts.clone(),
            },
            &options,
            conjunto,
        ),
        "api" | "http" => ejecutar_auditoria(
            &ConectorAPI {
                url:           conexion.to_string(),
                headers:       vec![("Accept".to_string(), "application/json".to_string())],
                nombre_fuente: "API".to_string(),
                timestamp:     ts.clone(),
            },
            &options,
            conjunto,
        ),
        otro => Err(format!("Fuente desconocida '{}'. Usar: csv | excel | sql | api", otro)),
    };

    let r = match resultado {
        Ok(v)  => v,
        Err(e) => {
            eprintln!("{} {}", "ERROR".red().bold(), e);
            std::process::exit(1);
        }
    };

    // ── Mostrar resultado ─────────────────────────────────────
    let sep = "═".repeat(70);
    println!("{}", sep.cyan());
    println!("{}", "  AUDITORÍA COMPLETADA".cyan().bold());
    println!("{}", sep.cyan());
    println!("  Audit ID     : {}", r.evidencia.audit_id.yellow());
    println!("  Timestamp    : {}", r.evidencia.timestamp);
    println!("  Reglas       : {} v{}",
        r.evidencia.version_reglas.conjunto,
        r.evidencia.version_reglas.version);
    println!("  Entidades    : {}", r.datos.total.to_string().green());
    println!("  Anomalías    : {}", r.anomalias.len().to_string().yellow());
    println!("  Hash canón.  : {}", r.hash_canonico.green());
    println!("  Chain hash   : {}", r.evidence_chain_hash.dimmed());
    println!();

    if r.anomalias.is_empty() {
        println!("  {} Sin anomalías detectadas", "OK".green().bold());
    } else {
        for a in &r.anomalias {
            let sev = match a.severidad {
                Severidad::Critico => "CRIT".red().bold(),
                Severidad::Medio   => "MED ".yellow().bold(),
                Severidad::Bajo    => "BAJO".white().bold(),
            };
            println!("  [{}] {} | {} | {} | actual='{}' esperado='{}'",
                sev, a.regla_id, a.entidad_id, a.campo,
                a.valor_actual, a.valor_esperado);
        }
    }

    // ── Exportar PDF + JSON ───────────────────────────────────
    if export {
        println!();
        match generar_evidencia(&r.evidencia, &r.hash_canonico, salida) {
            Ok((pdf, json)) => {
                println!("  {} PDF  : {}", "EXPORT".green().bold(), pdf);
                println!("  {} JSON : {}", "EXPORT".green().bold(), json);
                println!();
                println!("  Para verificar:");
                println!("  axyom verify {} {}", json, r.hash_canonico);
            }
            Err(e) => {
                eprintln!("{} {}", "ERROR exportando".red().bold(), e);
                std::process::exit(1);
            }
        }
    } else {
        println!();
        println!("  Usar --export para generar PDF + JSON verificable");
    }

    println!("{}", sep.cyan());
    println!("  AXYOM v{} | 0 = 0 | Chile 2026", VERSION_MOTOR);
    println!("{}", sep.cyan());
}
