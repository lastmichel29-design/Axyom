@echo off
title AXYOM v7

cd /d "%~dp0"

echo.
echo AXYOM v7.0 - Compilando...
echo.

rustc --version >nul 2>&1
if %errorlevel% neq 0 (
    echo ERROR: Rust no esta instalado.
    echo Instala desde: https://rustup.rs
    pause
    exit /b 1
)

echo Configurando toolchain Windows...
rustup default stable-x86_64-pc-windows-msvc >nul 2>&1
rustup component add llvm-tools >nul 2>&1
echo OK

echo.
echo Compilando... espera 5-10 minutos la primera vez
echo.
cargo build --release

if not exist "target\release\axyom.exe" (
    echo.
    echo ERROR: No compilo. Pega el error arriba en el chat.
    pause
    exit /b 1
)

echo.
echo ============================================
echo COMPILADO OK - Corriendo demo...
echo ============================================
echo.

target\release\axyom.exe audit --fuente csv --conexion demo_trabajadores.csv --reglas mineria --export

echo.
echo ============================================
echo Listo. Revisa la carpeta:
echo   reporte.pdf
echo   reporte_evidencia.json
echo ============================================
echo.
pause
