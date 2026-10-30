# Viewport e geometria interativa

O modo Ambiente usa rasterização wgpu com teste de profundidade. O renderer mantém um buffer das malhas originais e matrizes por objeto: navegar e selecionar não reenviam os vértices. Transformações de posição, rotação e escala atualizam matrizes, incluindo a inversa transposta das normais. O picking mantém a projeção do integrador CPU.

Durante um arraste, a cena CPU fica preservada. Ao soltar, a transformação é confirmada uma vez na geometria e na BVH; Esc descarta a prévia, preservando o histórico. A matriz temporária também posiciona os handles. Esta estratégia adapta a separação entre malha persistente, câmera e desenho por callback do SolidCraft, com atribuição e licença em ATTRIBUTION.md.

No modo Tempo real, a movimentação exibe a prévia sólida; após 160 ms sem edição, volta ao path tracing CPU. O render final continua CPU e confirma qualquer transformação pendente antes de capturar a cena.

## Limites desta etapa

- A visualização sólida usa cor base e iluminação simples; mapas PBR/texturas continuam nos modos de render CPU.
- A BVH global ainda é reconstruída ao confirmar a edição. Malhas grandes podem pausar nesse momento; instâncias e BVH por objeto permanecem pendentes.
- O kernel CAD/BRep/truck do SolidCraft não foi integrado. Booleans, superfícies e tesselação paramétrica exigem uma próxima etapa com interfaces e testes próprios.
- Não há benchmark de FPS em placa física nem validação manual do aplicativo no Windows. Testes offscreen validam pixels, profundidade e cache; não equivalem à experiência no computador do usuário.
- `ABRIR_MODO_COMPATIBILIDADE.bat` inicia a prévia CPU antiga (`--cpu-viewport`). O rodapé identifica o adaptador utilizado no modo wgpu; um adaptador de software não é aceleração física.

## Validação reproduzível

```sh
cargo test --all-targets
cargo test --features desktop-ui --all-targets
RV_REQUIRE_GPU=1 cargo test --features desktop-ui --test gpu_viewport -- --nocapture
```

O teste offscreen permite ausência de adaptador somente sem `RV_REQUIRE_GPU`. Ele compara profundidade com ordem de triângulos invertida, seleção, órbita, transformação e contador de uploads. Testes da aplicação executam 1.000 atualizações por gesto e verificam que o Arc da cena não muda até a confirmação, com undo/redo e cancelamento.

Resultados locais desta revisão: 79 testes passaram com `desktop-ui`, 76 sem a interface; Clippy do núcleo aprovado. O teste wgpu obrigatório desenhou por Mesa llvmpipe/OpenGL (adaptador de software), com inspeção da imagem de teste. Isso não mede aceleração em placa física.
