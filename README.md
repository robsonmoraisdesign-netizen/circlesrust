# CirclesRust — tradução progressiva do Blender Cycles para Rust

**Motor original usado como referência:** [clonecircles](https://github.com/robsonmoraisdesign-netizen/clonecircles), branch `main`, commit `3eb35f85996baa6c1b2e19bf297b22745c85e321`. O repositório original é C/C++. Aqui a conversão é incremental: **ainda não existe um port completo do Cycles**.

## Nova fase: qualidade, normais suaves, MIS e importação OBJ

O renderizador Rust CPU agora inclui interpolação de normais por vértice, interseção de raios com a luz retangular, MIS aproximado entre iluminação direta/BSDF/ambiente, pós-filtro guiado por normais e profundidade, e importação de geometria OBJ estática.

Use:

~~~powershell
.\circlesrust-preview.exe Render.png 1920 1080 128 --denoise 3
.\circlesrust-preview.exe MeuModelo.png 1280 720 128 --obj modelo.obj --denoise 3
~~~

O pacote Windows de testes contém a imagem Full HD e a comparação **com versus sem denoise**. Consulte [detalhes técnicos e limitações da fase](docs/QUALIDADE_MIS_DENOISE_OBJ.md).

**O código atual ainda funciona em CPU.** Não há GPU Rust, OIDN/OptiX, shader graph, MTL/UV ou importação completa do Cycles.

## Imagens para avaliação em resolução adequada

**O GitHub Actions agora inclui uma imagem Full HD 1920 × 1080, com 64 amostras por pixel**, no artefato Windows, além de imagens pequenas para testes automáticos. O script Windows gera 1920 × 1080 em 128 amostras. Os testes PNG/BMP pequenos não devem ser usados para avaliar a qualidade da renderização.

Consulte [orientações para resolução, samples, CPU/GPU e avaliação visual](docs/AVALIAR_QUALIDADE_FULLHD.md).

## Novo: renderização CPU com múltiplas amostras

O antigo executável BMP tinha uma amostra por pixel, uma luz direcional e sombreado simples. A nova versão executa um integrador de caminhos em Rust com:

- Amostras por pixel (anti-aliasing com jitter subpixel);
- Luz retangular de área, sombras suaves e amostragem direta;
- Múltiplos ressaltos, iluminação indireta e roleta russa;
- BSDF difusa e microfacetas GGX metálicas/rugosas, com Fresnel Schlick;
- Cena de testes com piso, fundo, cubos e esfera metálica;
- Paralelismo por múltiplos núcleos da CPU e sementes reproduzíveis;
- Exposição, tonalização tipo ACES (aproximada) e sRGB;
- Exportação PNG e BMP sem bibliotecas externas de imagem.

**Limitações:** não há shader graph Cycles, MIS completo, volumes, instâncias, texturas, materiais por objetos importados, denoise, HDRI, CUDA, OptiX, HIP, Metal ou outras GPUs. O integrador usa algoritmos de path tracing inspirados nos módulos originais, mas não é uma reprodução integral do integrador Cycles.

## Baixar Windows

Acesse a aba **Actions**, escolha uma execução bem-sucedida e baixe o artifact `circlesrust-pathtracer-windows`. Após extrair o ZIP, execute o EXE via Prompt de Comando ou abra o script `RENDER_QUALIDADE_WINDOWS.bat` que vem no pacote.

## Gerar imagens

Qualidade padrão, PNG:

```bat
circlesrust-preview.exe MinhaImagem.png 960 540 64
```

Mais amostras (menos ruído, mais processamento):

```bat
circlesrust-preview.exe MinhaImagem_AltaQualidade.png 1280 720 128
```

Também existe o formato com parâmetros nomeados:

```bat
circlesrust-preview.exe --output cena.png --width 960 --height 540 --samples 96 --bounces 6 --exposure 1.2 --seed 17
```

Comando antigo continua funcionando: `circlesrust-preview.exe teste.bmp 960 540`.

As imagens geradas nesta fase mostram uma **cena demonstrativa interna**, ainda não importam modelos do usuário.

## Mapeamento do Cycles

| Código original | Rust | Situação |
| --- | --- | --- |
| `src/util/hash.h` | `src/util/hash.rs` | Jenkins e PCG |
| `src/util/math_float3.h` | `src/util/math.rs` | Vetores |
| `src/util/math_intersect.h` e `src/kernel/geom/triangle_intersect.h` | `src/kernel/geometry.rs` | Interseção de raios e triângulos |
| `src/util/boundbox.h`, `src/bvh/build.cpp`, `src/bvh/bvh2.cpp` | `src/bvh/mod.rs` | BVH binária CPU inicial, sem SAH |
| `src/kernel/closure/bsdf_diffuse.h` | `src/integrator/mod.rs` | BSDF difusa e amostragem coseno/pi |
| `src/kernel/light/area.h`, `src/kernel/integrator/shade_surface.h`, `src/kernel/integrator/path_state.h` | `src/integrator/mod.rs` | Área de luz e primeira aproximação de transporte indireto |
| `src/scene/*` | `src/scene/mod.rs` | Cenário demonstrativo, não conversão completa |
| `src/kernel/film/*` | `src/film/mod.rs` | PNG/BMP independente |

Veja também [detalhamento técnico](docs/TRANSPORTE_LUZ_CPU.md).

## Compilar e testar

```sh
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo run --release --bin circlesrust-preview -- render.png 960 540 64
```

O workflow compila Windows e Linux e publica o executável Windows como artefato. **Releases não aparecem automaticamente**: publicar um Release é outro processo.

## Layout RV Studio

O [layout HTML corrigido](ui/RV_Studio_Render_Layout_Corrigido.html) segue o visual aprovado, com tokens centrais de aparência e temas claro/escuro. Ainda não há comunicação da interface com o motor de imagem Rust.

## Próximas fases de migração

1. BVH2 com SAH/binning, instâncias e motion blur;
2. Luzes múltiplas e MIS, shader graph, texturas e importação de cena;
3. Film buffers progressivos, denoise e gerenciamento de cores;
4. Integração dos dispositivos GPU do Cycles, interface desktop Rust e empacotamento.
