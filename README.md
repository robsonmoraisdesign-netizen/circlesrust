# CirclesRust — conversão gradual do Cycles para Rust

## Situação real

Este projeto será uma migração modular do **Blender Cycles** (C/C++) para **Rust**. A cópia completa do código-fonte original ainda não aparece nas branches deste repositório; esta primeira alteração se baseia nos módulos originais **oficiais**, não em um renderizador substituto.

Referência original: [Blender Cycles, commit 3eb35f85996baa6c1b2e19bf297b22745c85e321](https://github.com/blender/cycles/tree/3eb35f85996baa6c1b2e19bf297b22745c85e321).

| Módulo original | Tradução Rust |
| --- | --- |
| `src/util/hash.h` | `src/util/hash.rs`: Jenkins Lookup3, PCG 2D/3D/4D e hashing em float |
| `src/kernel/types.h` | `src/kernel/types.rs`: dimensões e padrões de amostragem, visibilidade de raios e constantes |

Os arquivos convertidos identificam o trabalho original e as modificações; licença Apache 2.0 em `LICENSE`.

## Validação

Instale Rust estável e rode `cargo test` e `cargo clippy --all-targets -- -D warnings`. O GitHub Actions executa em Windows e Linux.

## Próximos módulos

- Matemática e raios, scene/camera e scene/shader;
- BVH, kernel integrator e dispositivos CPU;
- Migração dos backends GPU do Cycles, com validação e performance;
- Interface desktop Rust seguindo layout já aprovado no RV Studio Render e tema centralizado.

**Esta primeira versão é apenas uma biblioteca Rust de componentes portados. Ainda não renderiza imagens, não executa na GPU e não equivale ao Cycles completo.** É necessário importar o código fonte original no repositório para validar a conversão de todos os subsistemas.
