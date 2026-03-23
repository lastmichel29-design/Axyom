# AXYOM Minería v7.0

> **Motor de auditoría laboral para la industria minera chilena**  
> Detecta incumplimientos del DS 44/2024 y Ley 16.744 en menos de 30 minutos.  
> Genera PDF legal con hash SHA3-256 verificable en tribunal.

[![Rust](https://img.shields.io/badge/Rust-1.85+-orange?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/version-7.0.0-blue?style=flat-square)](https://github.com/lastmichel29-design/Axyom/releases)
[![Chile](https://img.shields.io/badge/hecho%20en-Chile%202026-brightgreen?style=flat-square)]()

---

## ¿Qué hace AXYOM?

AXYOM se conecta a tus datos de trabajadores **(solo lectura, nunca escribe)** y detecta automáticamente:

| Código | Regla | Severidad | Normativa |
|--------|-------|-----------|-----------|
| M001 | EPP no asignado | CRÍTICO | DS 44/2024 art. 37 |
| M002 | Fecha de contrato vacía | MEDIA | Ley 16.744 |
| M003 | Examen médico no APROBADO | CRÍTICO | DS 594 |
| M004 | Capacitación sin completar | MEDIA | DS 40 |
| MC001 | Examen OK pero sin capacitación | MEDIA | DS 40 |
| MC002 | Contrato existe pero sin EPP | CRÍTICO | DS 44/2024 |

**Una multa SERNAGEOMIN por incumplimiento grave = $50M - $500M CLP.
AXYOM la detecta antes de que llegue el inspector.**

---

## Demo rápido

```bash
axyom.exe audit --fuente csv --conexion trabajadores.csv --reglas mineria --export
```

Resultado:
```
AUDITORÍA COMPLETADA
  Entidades : 7
  Anomalías : 10
  Hash SHA3 : 098a821607d5ed0c07d854e093bea8cf...

  [CRIT] M001 | Maria Lopez  — EPP no asignado
  [CRIT] M003 | Carlos Soto  — examen PENDIENTE
  [MED]  M002 | Ana Torres   — contrato vacío
  [CRIT] M001 | Rosa Muñoz   — EPP no asignado

  EXPORT PDF  : reporte.pdf
  EXPORT JSON : reporte_evidencia.json
```

---

## Características v7.0

- **Reglas YAML externas** — cambia normativa sin recompilar
- **JSON canónico** — hash SHA3 idéntico en cualquier plataforma
- **Streaming CSV** — procesa millones de filas sin agotar RAM
- **Paralelismo rayon** — evaluación multi-core
- **API REST** — endpoints HTTP para integración
- **4 conectores** — CSV, Excel, SQLite, API JSON
- **PDF multipágina** — reporte legal con cadena de custodia
- **Hash verificable** — `axyom verify evidencia.json <hash>`

---

## Instalación Windows

```powershell
# Doble clic en 1_INSTALAR_TODO.bat
# O manualmente:
rustup default stable-x86_64-pc-windows-gnu
rustup component add llvm-tools
set PATH=C:\msys64\mingw64\bin;%PATH%
cargo build --release
```

---

## Comandos

```bash
axyom.exe info
axyom.exe audit --fuente csv --conexion datos.csv --reglas mineria --export
axyom.exe audit --fuente excel --conexion datos.xlsx --reglas mineria --export
axyom.exe audit --fuente csv --conexion datos.csv --ruta-reglas rules/mineria.yaml --export
axyom.exe verify reporte_evidencia.json <hash>
axyom.exe api --puerto 8080
```

---

## API REST

```bash
axyom.exe api --puerto 8080

curl -X POST http://127.0.0.1:8080/audit \
  -H "Content-Type: application/json" \
  -d '{"csv": "trabajadores.csv", "reglas": "mineria"}'
```

---

## Estadísticas

| Métrica | Valor |
|---------|-------|
| Líneas de código Rust | 3.049 |
| Teoremas Coq verificados | 128 |
| Admits en lógica core | 0 |
| Tests de integración | 13 |
| Conectores de datos | 4 |

---

## Normativa

- DS 44/2024 MINTRAB — Reglamento de Seguridad Minera
- Ley 16.744 — Accidentes del Trabajo
- DS 594 — Condiciones Sanitarias
- DS 40 — Prevención de Riesgos
- SHA3-256 — Cadena de custodia verificable en tribunal

---

## Autor

**Michel Antonio Durán Cornejo**  
Santiago, Chile — 2026  
[github.com/lastmichel29-design/Axyom](https://github.com/lastmichel29-design/Axyom)

---

*Axioma: 0 = 0 | "Hemos ganado sin que nadie muera"*
