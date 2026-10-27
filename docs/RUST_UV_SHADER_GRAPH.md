# Marco de texturas e grafo de nós

## Referência técnica

clonecircles/src/scene/image.cpp, clonecircles/src/scene/shader_nodes.cpp, clonecircles/src/kernel/svm e clonecircles/src/kernel/geom/triangle.h são referências arquitetônicas. O código escrito nesta etapa é nativo Rust e não compila esses módulos C++.

## Implementações desta etapa

- src/shader/mod.rs: tipo Uv, interpolação baricêntrica, ImageTexture em RGB linear, amostragem bilinear com wrap, sRGB 8-bit -> linear e grafo de cor acíclico com até 64 nós.
- src/scene/obj.rs: leitor Wavefront OBJ (v, vt, vn), com face triangulada e uv preservado por canto.
- src/scene/mtl.rs: leitor map_Kd além de Kd, Pm, Pr e Ns; o caminho é resolvido relativamente ao arquivo MTL pela CLI.
- src/integrator/mod.rs: seleciona a BSDF material por triângulo e avalia a cor do shader no UV baricêntrico de cada hit; preserva materiais sem textura e cenas anteriores.
- src/scene/mod.rs: buffer de UV alinhado ao triângulo, inclusive nas faces procedurais, com None quando não existe mapeamento.
- src/bin/circlesrust-preview.rs: carrega imagens uma vez, não durante a renderização. Cria um grafo Constant(Kd) x Image(map_Kd) por material.
- Cargo.toml: dependência da crate Rust image com decodificadores PNG, JPEG e PNM; não usa motor C++.

## O que os testes verificam

- UV do formato OBJ com triangulação e interpolação baricêntrica.
- Correspondência entre nome MTL, map_Kd e material selecionado.
- Conversão sRGB, wrap 0..1, inversão V e filtragem bilinear.
- Validação de referências e ciclos no grafo.
- O render efetivamente gera pixels diferentes com e sem textura, mantendo mesma seed e geometria.
- Execução CLI com imagem PNM no CI em Windows/Linux.

## Limitações de produção

Sem Principled BSDF de Cycles, node editor, imagem EXR/HDRI, texturas 16-bit/float, UV sets múltiplos, mapas normais, rugosidade e bump, alfa/transparência, filtragem anisotrópica ou mipmaps. A crate image é Rust; a integração GPU continua pendente.

## Percentual de migração

A métrica indicada nas respostas compara exclusivamente bytes dos arquivos .rs em src/ com bytes dos arquivos C/C++ principais do clonecircles. Essa razão mede volume de código, não equivalência funcional ou porcentagem de features portadas. Deve ser recalculada após cada mudança integrada à main.
