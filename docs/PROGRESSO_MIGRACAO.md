# Progresso da migração em Rust

## Percentual desta entrega

**1,47% — indicador de volume do núcleo**, e não conclusão funcional do Cycles.

- Numerador: 2.585 linhas não vazias dos arquivos `.rs` em `src/bvh`, `film`, `integrator`, `kernel`, `scene`, `shader`, `util`.
- Denominador: 176.087 linhas não vazias de arquivos `.c`, `.cc`, `.cpp`, `.h`, `.hpp`, `.cu`, `.cuh`, `.metal`, `.mm`, `.cl` em `clonecircles/src`.
- Referência fixa: `robsonmoraisdesign-netizen/clonecircles`, commit `3eb35f85996baa6c1b2e19bf297b22745c85e321`.
- Fórmula: `100 × linhas Rust do núcleo / linhas da referência`.
- Baseline da entrega anterior `8d7e5dc`, sob o MESMO critério: 2.493 / 176.087 = **1,42%**. Atual: **1,47%**.
- Comentários dentro dos arquivos contados entram na contagem. Interface, CLI, fontes de testes, HTML, documentação e build scripts não entram no numerador. Referências de módulo de teste dentro do núcleo entram como linhas de declaração, não o arquivo do teste.

Linhas Rust e C++ não equivalem em função ou esforço. Este indicador pode cair quando há refatoração e não deve orientar quantidade de código. Não é possível afirmar que o software tenha 1,47% das funcionalidades do Cycles: ainda não existe inventário funcional completo com aceites auditados. A estimativa antiga por volume não tinha este critério fixado e não deve ser comparada diretamente. Usar esta mesma base em próximas entregas, sem trocar o denominador para aumentar o número.

Reprodução em Rust:

```sh
cargo run --bin migration-progress -- /caminho/clonecircles-reference
```

O contador rejeita um checkout de referência diferente ou com modificações em `src`.

## Funcionalidade entregue e verificável

| Área | Situação |
|---|---|
| Aplicativo desktop Rust/egui, temas e painéis | Implementado; não é frontend JavaScript |
| OBJ/MTL, grupos, UV e mapas PBR básicos | Implementado parcialmente; não equivale ao importador Blender |
| BVH SAH, triângulos, normais suaves | Implementado para malhas estáticas |
| Render CPU, BSDF difusa/GGX, luz retangular e MIS básico | Implementado parcialmente |
| HDRI float, amostragem por importância e PDF correspondente | Nova etapa implementada e testada |
| Seleção, pan/órbita/zoom, G/R/S numérico | Implementado |
| Handles mover/escalar, desfazer/refazer/cancelar | Implementado; interação visual Windows ainda precisa validação |
| PNG/BMP, prévia sólida/tempo real CPU, filtro de ruído básico | Implementado; não é OIDN/OptiX |
| Anéis de rotação, snapping, hierarquia e instâncias | Pendentes |
| GPU do viewport sólido, matrizes e cache de malhas | Implementado nesta etapa; validação de placa física pendente |
| GPU de path tracing, CAD/BRep SolidCraft | Pendentes |
| Principled completo, transmissão/refração, volumes, cabelo e motion blur | Pendentes |
| EXR, OCIO/AgX, light tree, portais, shader graph completo | Pendentes |
| Salvamento/abertura de projeto e biblioteca de materiais | Pendentes |

## Testes desta etapa

- Distribuição de probabilidade normalizada e consistente entre amostra e avaliação.
- Mapas constantes, pretos, HDR extremo, 1×1, dimensões não divisíveis pelo limite, polos e entradas inválidas.
- Hotspot selecionado proporcionalmente à energia.
- Estimador contra quadratura independente da radiância bilinear: erro médio relativo < 3%; erro quadrático das amostras < metade do uniforme no cenário sintético.
- Combinação MIS de ambiente/BSDF versus quadratura independente, materiais dielétrico e metálico: tolerância relativa de 2%.
- Desfazer/refazer reproduz pixels do render antes/depois de transformação combinada.
- Guarda de arquitetura rejeita arquivos de implementação JavaScript/TypeScript/HTML/Python/C++ em `src`; não pretende auditar todas as dependências transitivas.
- 74 testes passaram na suíte completa local; Clippy sem avisos do núcleo e verificação de compilação desktop aprovada (3 avisos preexistentes da UI); status Windows deve ser informado no momento da entrega.

## Regras de entrega

Sempre informar percentual/métrica, mudanças reais, evidência de testes, limitações e o que falta. Não trocar Rust por JavaScript para fechar uma etapa. O HTML arquivado é apenas referência visual. Executável e fonte devem corresponder à mesma revisão; Actions em PR pode empacotar o commit de teste de merge, preservado no comentário de `Codigo-Fonte.zip`.

## Etapa SolidCraft / viewport

Indicador permanece **1,47%** (2.585 / 176.087). Câmera, renderer e controle de gesto ficam em `src/desktop`, fora do numerador fixo; não inflar o progresso pelo código de interface. Nova integração documentada em `SOLIDCRAFT_VIEWPORT.md`; origem e licença em `ATTRIBUTION.md`. Testes adicionais: matrizes versus geometria CPU, projeção versus picking, pixels offscreen, profundidade, seleção, órbita, cache e 1.000 atualizações por gesto sem reconstrução da cena. Resultados da execução corrente e CI devem acompanhar a entrega.

Validação local desta etapa: **79 testes com desktop-ui**, 76 no núcleo/CLI sem a interface; Clippy do núcleo aprovado. Pipeline wgpu validado obrigatoriamente com Mesa llvmpipe/OpenGL, adaptador de software. Não representa medição em GPU física nem interação manual Windows.
