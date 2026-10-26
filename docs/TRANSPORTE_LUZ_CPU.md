# CPU Path Tracer — renderização de amostras progressivas

Referência de código: [clonecircles, commit 3eb35f8](https://github.com/robsonmoraisdesign-netizen/clonecircles/tree/3eb35f85996baa6c1b2e19bf297b22745c85e321).

Esta versão vai além do raycasting simples inicial, mas **não é a reescrita completa do Cycles**.

## Arquitetura

1. A câmera cria raios com jitter subpixel, com sementes determinísticas para cada pixel e amostra.
2. A BVH CPU encontra a primeira interseção com a malha de triângulos.
3. Em cada superfície, há amostragem direta da luz retangular; um raio de sombra verifica oclusão.
4. A BSDF (difusa ou GGX) amostra uma nova direção, atualizando o throughput do caminho com f × cos(theta) / pdf.
5. O caminho pode atravessar vários ressaltos; após alguns, roleta russa controla o custo.
6. Quando o raio não atinge geometria, a contribuição do ambiente é acumulada.
7. As amostras são somadas em RGB linear, tonalizadas com aproximação de ACES e convertidas para sRGB.
8. O resultado é escrito em PNG lossless ou BMP. A codificação PNG usa DEFLATE sem compressão.

## Fontes e limites

- `src/kernel/closure/bsdf_diffuse.h`: BSDF Lambertiana e distribuição de hemisfério coseno/pi; conteúdo original tem atribuição BSD-3-Clause, enquanto componentes próprios do novo código são distribuídos segundo as licenças aplicáveis;
- `src/kernel/light/area.h`: conceitos da iluminação por área; esta etapa amostra uniformemente um retângulo com geometria cos(luz) / r²;
- `src/kernel/integrator/shade_surface.h` e `path_state.h`: estado, throughput e propagação do caminho;
- `src/kernel/integrator/shade_background.h`: conceito de iluminação de fundo;
- `src/util/hash.h`: sementes PCG/Jenkins.

A aproximação de microfacetas GGX usa Fresnel Schlick e Smith; **não equivale ao shader Principled BSDF** do Cycles. Não há múltipla amostragem de importância de várias luzes, nem o integrador completo de volumes, caustics e path guiding.

## Desempenho

A imagem é processada em blocos horizontais por múltiplas threads nativas de CPU, sem usar bibliotecas externas. O custo cresce com a resolução, a quantidade de amostras e o número de ressaltos. A qualidade melhora com mais amostras, mas ainda haverá ruído sem denoiser.

## Validação

- CI roda Clippy, testes da BVH, testes determinísticos de imagem e build de release em Windows e Linux.
- Os testes comparam renderizações com mesma semente e verificam formato PNG/BMP.
- O workflow gera uma pequena imagem PNG e outra BMP para smoke tests.
- A comparação física e visual com o Cycles original ainda precisa ser implementada.

## Uso

```sh
cargo run --release --bin circlesrust-preview -- minha_imagem.png 1280 720 128
```
