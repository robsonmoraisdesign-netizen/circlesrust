# SolidCraft: origem da adaptação

Esta etapa adapta partes do SolidCraft (MIT) para o viewport nativo do CirclesRust:

- `src/desktop/mesh_view.rs`: representação column-major, multiplicação e projeção de câmera, adaptadas de `crates/render/src/camera.rs`, blob `6e1d38f7bda494a2c2a990df20983a3fc15597e1`.
- `src/desktop/gpu_view.rs`: instalação de recursos persistentes e callbacks egui `prepare`/`paint`, adaptados de `crates/ui-egui/src/gpu.rs`, blob `222938e41756885507e12673c62cfdc54f0d56b5`.

Fonte: https://github.com/robsonmoraisdesign-netizen/solidcraftclone/tree/batch/a

Copyright (c) 2026 SolidCraft contributors. Licença integral em `third_party/SOLIDCRAFT-MIT.txt` (blob de origem `bd0e0f2680c2ba9d654d3e57e92fe1a77837f641`). Os identificadores acima registram os arquivos efetivamente consultados; a branch pode mudar.

A adaptação usa egui 0.32 / wgpu 25 e a geometria, seleção, tema e câmera existentes do CirclesRust. Não incorpora integralmente o kernel CAD/BRep/truck, tesselação GPU ou todas as ferramentas do SolidCraft. O shader de visualização e o protocolo de transformação de malhas foram implementados para este aplicativo em Rust/WGSL.
