@echo off
setlocal
cd /d "%~dp0"
title CirclesRust - Render Full HD com MIS e Denoise
echo ============================================
echo CirclesRust - Renderizacao CPU em Full HD
echo Normais suaves, MIS e denoise guiado
echo ============================================
echo Resolucao: 1920 x 1080
echo Amostras por pixel: 128
echo Denoise: 3 passos
echo ATENCAO: o motor atual ainda usa CPU, nao GPU.
echo.
circlesrust-preview.exe Render_FullHD_128_Amostras.png 1920 1080 128 --denoise 3
if errorlevel 1 (
  echo Falha na renderizacao Rust.
) else (
  echo Render concluido: Render_FullHD_128_Amostras.png
)
pause
