# CirclesRust — tradução progressiva do Blender Cycles para Rust

**Motor original usado como referência:** [clonecircles](https://github.com/robsonmoraisdesign-netizen/clonecircles), branch `main`, commit `3eb35f85996baa6c1b2e19bf297b22745c85e321`. O repositório original é C/C++. Aqui a conversão é incremental: **ainda não existe um port completo do Cycles**.

## Novo: aplicativo Windows com interface gráfica Rust

O módulo experimental `circlesrust-studio.exe` agora possui interface desktop **egui/eframe (Rust)**, não só o utilitário CLI. A organização segue as áreas do RV Studio Render aprovado: ambiente/objetos à esquerda, imagem central, câmera inferior e materiais à direita, com temas centralizados.

O botão **RENDERIZAR** usa o núcleo Rust CPU real e salva a imagem. A opção `desktop-ui` permite compilar o aplicativo Windows:

~~~sh
cargo build --release --features desktop-ui --bin circlesrust-studio
~~~

A versão CLI antiga permanece inalterada. Consulte [instruções e limitações da UI nativa](docs/GUI_DESKTOP_RUST.md). O viewport central ainda não é um modelador 3D interativo; GPU não está integrada.

## Iluminação por mapa panorâmico

Você também pode substituir o céu procedural por uma imagem panorâmica equiretangular. Arquivos Radiance **.hdr (RGBE)** são decodificados em luz RGB linear de 32 bits sem cortar valores acima de 1; panoramas PNG/JPEG/PNM são convertidos de sRGB para linear.

~~~powershell
.\circlesrust-preview.exe Render_HDR.png 1920 1080 128 --obj modelo.obj --environment studio.hdr --env-strength 1.0 --denoise 3
~~~

O arquivo é carregado diretamente pelo programa Rust, sem precisar indicar caminhos para Blender, LuxCore ou DLLs externas. A iluminação usa o mapa na contribuição do ambiente para os raios primários e indiretos e na estimativa de luz ambiente.

**Limitações:** a amostragem panorâmica nesta fase é hemisférica, sem importância baseada na luminosidade dos texels do HDRI. Hotspots pequenos podem ficar ruidosos. Suporte formal de EXR, gerenciamento OCIO/AgX e GPU continuam pendentes.

## Fase atual — mapas de rugosidade, metalicidade e normais em Rust

A versão nova aceita mapas PBR independentes referenciados no MTL: map_Pr para rugosidade, map_Pm para metalicidade e norm/map_Kn para normais tangenciais OpenGL (+Y). Os canais são decodificados em espaço **NonColor**, sem aplicação incorreta da curva de gamma sRGB. O canal vermelho de cada mapa escalar substitui a constante de roughness/metalness, enquanto o mapa de normal altera a direção usada pela BSDF.

Exemplo de MTL:

~~~mtl
newmtl metal_escovado
Kd 0.75 0.72 0.62
map_Kd cor.png
map_Pr rugosidade.png
map_Pm metalicidade.png
norm normal_opengl.png
~~~

A localização das imagens é relativa ao MTL; o objeto precisa de coordenadas UV válidas. A geometria permanece inalterada, e o normal map **não** é deslocamento real nem height map. bump/map_Bump não é interpretado como normal tangent-space.

~~~powershell
.\circlesrust-preview.exe Modelo_PBR.png 1280 720 128 --obj modelo.obj --denoise 3
~~~

**Limites:** o motor permanece CPU; os mapas são 8 bits RGB e não há 16-bit/float, múltiplos UV sets, UDIM, mipmaps, filtragem anisotrópica, Principled completo ou backends GPU. As funcionalidades descritas são uma implementação própria em Rust guiada pelo Cycles, não uma cópia direta de todo o motor.

## Nova etapa — UV, texturas e nós de cor nativos em Rust

Agora a malha OBJ preserva coordenadas UV (vt e f v/vt/vn) e os arquivos MTL podem referenciar mapas de cor map_Kd. Texturas PNG, JPEG e PNM são carregadas em memória, convertidas de sRGB para RGB linear e filtradas bilinearmente nas coordenadas UV de cada interseção.

Um primeiro grafo de nós de cor totalmente em Rust permite combinar Constant, Image, Checker, Multiply e Mix. Para map_Kd, uma cadeia Constant(Kd) x Image modula a cor base. O shader é avaliado a cada hit de raio, não apenas na importação.

~~~powershell
.\circlesrust-preview.exe ModeloTexturizado.png 1280 720 128 --obj meu_modelo.obj --denoise 3
~~~

Se o OBJ possuir mtllib meu_modelo.mtl e o MTL possuir map_Kd textura.png, o arquivo da textura é localizado relativamente ao MTL, sem configurar diretórios externos do motor.

**Limitações:** não é o shader graph completo do Blender Cycles. Ainda faltam Principled BSDF, normal/roughness maps, transmissões, mapas HDRI, shader nodes procedurais avançados, OSL/SVM, GPU e mipmapping. O filtro bilinear usa texturas com repetição UV, sem derivadas nem filtragem anisotrópica.

## Fase atual — BVH SAH e materiais OBJ/MTL em Rust

- **BVH SAH:** a divisão por mediana foi substituída por amostragem em 16 bins e avaliação de custo de área superficial nos três eixos, seguindo a estrutura de `clonecircles/src/bvh/binning.cpp`. Se a partição degenerar, o algoritmo continua com fallback por mediana.
- **Materiais OBJ/MTL:** o leitor OBJ agora preserva `mtllib` e `usemtl`, associando as faces aos materiais carregados. O leitor MTL suporta `newmtl`, `Kd` (cor), `Ns` (rugosidade aproximada), `Pr` (rugosidade PBR) e `Pm` (metalicidade PBR).
- **CLI:** `--obj modelo.obj` procura automaticamente bibliotecas `.mtl` citadas no OBJ, na mesma pasta. Você também pode indicar `--mtl material.mtl` explicitamente.
- **Testes:** cenas com dois materiais, triângulos agrupados espacialmente, interseções da BVH versus força bruta e casos degenerados de partição.

Exemplos:

```powershell
.\circlesrust-preview.exe ModeloColorido.png 1280 720 128 --obj modelo.obj --denoise 3
.\circlesrust-preview.exe ModeloManual.png 1280 720 128 --obj modelo.obj --mtl modelo.mtl --denoise 3
```

**Importante:** continua sendo um executável 100% Rust em CPU. O código C++ original é somente referência, **não** é compilado nem carregado. O leitor MTL não implementa texturas, `map_Kd`, transparência, refração, normal maps nem nós de shaders do Cycles. A BVH ainda não suporta spatial splits, instâncias nem motion blur.

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
