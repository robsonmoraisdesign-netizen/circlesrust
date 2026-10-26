@echo off
setlocal
cd /d "%~dp0"
echo RV Studio Render - CirclesRust CPU Path Tracing
echo Gerando previa de alta qualidade com 64 amostras...
circlesrust-preview.exe Render_Qualidade.png 960 540 64
if errorlevel 1 (
  echo Erro ao executar o motor Rust.
) else (
  echo Imagem salva como Render_Qualidade.png
)
pause
