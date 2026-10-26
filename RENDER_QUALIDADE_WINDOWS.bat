@echo off
setlocal
cd /d "%~dp0"
title CirclesRust - Render Full HD
echo ============================================
echo CirclesRust - Renderizacao CPU em Full HD
echo ============================================
echo Resolucao: 1920 x 1080
echo Amostras por pixel: 128
echo Motor atual: CPU, nao GPU
echo O processamento pode ser demorado nesta qualidade.
echo.
circlesrust-preview.exe Render_FullHD_128_Amostras.png 1920 1080 128
if errorlevel 1 (
  echo Erro no render Rust.
) else (
  echo Render concluido: Render_FullHD_128_Amostras.png
)
pause
