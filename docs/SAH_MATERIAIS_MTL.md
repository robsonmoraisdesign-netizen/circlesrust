# Tradução incremental: binning SAH e materiais Wavefront

## Fontes de referência

- clonecircles/src/bvh/binning.cpp: avaliação por distribuição de primitivas em bins, prefixos de limites, SAH por eixo, fallback de divisão central/mediana.
- clonecircles/src/bvh/build.cpp: montagem da hierarquia usando as referências de geometria.
- clonecircles/src/scene/mesh.cpp, src/scene/shader.cpp: conceitos de geometria, material por face e configuração de shader (não são cópias integrais).

## Implementado em Rust

- src/bvh/sah.rs calcula o custo SAH usando 16 bins e varreduras em X/Y/Z; usa limites reais dos triângulos e centróides para seleção de bins.
- src/bvh/mod.rs usa essa partição, preserva os IDs originais dos triângulos e mantém fallback por mediana se não houver divisão válida.
- src/scene/obj.rs preserva mtllib e usemtl em cada triângulo após a triangulação de polígonos convexos.
- src/scene/mtl.rs lê Kd, Ns, Pm e Pr em materiais nomeados. Modelos antigos com Ns são aproximados para roughness via sqrt(2/(Ns+2)).
- src/scene/mod.rs associa os materiais nomeados ao integrador Rust usando IDs de triângulos; materiais ausentes usam padrão neutro.
- src/bin/circlesrust-preview.rs aceita --mtl e procura bibliotecas citadas pelo OBJ, sem impor caminhos externos para o motor.

## Limitações explícitas

- SAH por objetos estáticos não é o mesmo que SBVH/spatial split, GPU BVH8 ou BVH instanciada do Cycles.
- Materiais MTL não equivalem a Principled BSDF do Cycles: cor/rugosidade/metalicidade são propriedades simplificadas.
- Mapas de textura/UV, transmissão, volumetria, luzes complexas, materiais por nós, GPU Rust e integração da UI continuam pendentes.
- O total de bytes de código Rust dividido pelo C++ original é um indicador aproximado de volume, não uma medida confiável de paridade funcional.

## Validação

- Os testes novos verificam equivalência geométrica de buscas BVH e brute-force em centenas de raios em duas regiões de triângulos.
- Testes de faces OBJ com dois materiais, conversão MTL de roughness e verificações de erros.
- Smoke test de CLI com arquivos OBJ e MTL de fixtures do repositório e exportação PNG.
