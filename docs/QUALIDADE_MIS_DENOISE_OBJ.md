# Próxima fase CirclesRust: normais suaves, MIS, denoise e OBJ

## Base de referência

[clonecircles](https://github.com/robsonmoraisdesign-netizen/clonecircles), commit 3eb35f85996baa6c1b2e19bf297b22745c85e321.

Módulos originais consultados:
- src/kernel/geom/triangle.h: interpolação baricêntrica de normais por vértice, com retorno ao Ng geométrico
- src/kernel/integrator/shade_surface.h: shader normal versus offset geométrico de raios e emissão com pesos MIS
- src/kernel/light/area.h e src/kernel/light/sample.h: emissor retangular e densidade direcional de amostragem
- src/kernel/sample/mapping.h: distribuição de amostras cosseno-hemisféricas
- src/kernel/closure/bsdf_microfacet.h: objetivo funcional de microfacetas GGX

### Alterações implementadas no Rust

1. **Normais suaves:** Scene pode armazenar três normais por face. A interpolação usa barycentrics 1-u-v, u, v. A esfera procedural tem normais radiais, eliminando a maior parte dos artefatos de facetas no sombreado sem aumentar polígonos. Cube e piso continuam com normais planas.
2. **Luz refletida e MIS inicial:** raios BSDF enxergam uma fonte retangular emissiva, não somente recebem direct lighting. Foi adicionado um estimador MIS beta=2, combinando amostragem de área e BSDF, além de next-event estimation aproximada do ambiente com pdf cossenoidal.
3. **Pós-filtro a-trous opcional:** filtro de redução de ruído no espaço de imagem com guias de normal, profundidade e material. Esse filtro não é OIDN nem OptiX; pode reduzir ruído, mas também suavizar detalhes em amostragem baixa.
4. **OBJ estático inicial:** importação de v, vn e f, índices negativos, polígonos convexos triangulados em fan, normais por face do OBJ, autoajuste do tamanho no estúdio. MTL, textura, UV, materiais e animação OBJ ainda não são suportados.
5. **CLI:** --denoise [0..5], --no-denoise, --obj arquivo.obj. A forma antiga de passar o nome e resolução da imagem continua funcionando.
6. **Teste funcional de saída:** PNG Full HD 1920×1080 com 64 spp e denoise. Comparação lado a lado em 640×360 com denoise desligado/ligado, além de render de malha OBJ.

### Comandos

~~~powershell
.\circlesrust-preview.exe Render_FullHD.png 1920 1080 128 --denoise 3
.\circlesrust-preview.exe Render_Sem_Denoise.png 1920 1080 128 --no-denoise
.\circlesrust-preview.exe Meu_Modelo.png 1280 720 128 --obj modelo.obj --denoise 3
~~~

A resolução aceita permanece limitada a 4096×4096 pixels por validação do executável. Renderizações grandes aumentam o custo proporcionalmente aos pixels e amostras.

### Limitações que ainda impedem equivalência ao Cycles

- GPU no Rust: ainda não integrada (std::thread CPU; clonecircles original tem seus próprios backends)
- BVH: divisão pela mediana, não o SAH binning/spatial split de Cycles
- Integrador: sem graph shading nodes, shader Principled, SSS, volumetria e path guiding
- Importação de cenas: apenas OBJ geométrico básico neste estágio; sem MTL, OBJ UV e outros formatos
- Texturas, mapas HDRI, assets de produção: não implementados
- MIS: dois estimadores simplificados de luz/ambiente; não equivalentes aos algoritmos de Cycles
- Denoise: filtro a-trous local guiado, não IA nem pós-processamento do Cycles
- Interface RV Studio: HTML segue referência visual aprovada, ainda não recebe buffers do Rust

Não afirmar paridade visual, funcional ou de performance com Blender Cycles. A documentação do código identifica as funcionalidades correspondentes e suas limitações.
