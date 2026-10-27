# Etapa de materiais PBR via texturas Rust

Referências estudadas em clonecircles: `src/kernel/closure/bsdf_microfacet.h`, `src/scene/shader_nodes.cpp` e `src/scene/image.cpp`. Os originais incluem closures completas e nós mais ricos; **nenhuma paridade completa foi reivindicada**.

Implementado:
- `src/shader/mod.rs`: diferencia dados NonColor de albedo sRGB durante decodificação RGB8. Isso evita distorção do valor físico de roughness/metalness/normals.
- `src/shader/surface.rs`: amostra canais escalares e orientações tangenciais da normal, com derivação TBN por geometria e UV. Caso UV degenerado/triângulo sem normal map, conserva normal base. Mantém resultados na frente geométrica.
- `src/scene/mtl.rs`: lê map_Pr, map_Pm e norm/map_Kn e recusa sintaxe de parâmetros do mapa não suportados. map_Bump é height map e não é confundido com norm.
- `src/integrator/mod.rs`: resolve materiais e normais por hit; o integrador GGX/Lambert recebe os parâmetros modulados pelo material.
- `src/bin/circlesrust-preview.rs`: carrega imagens de todos os mapas antes da renderização, relativas ao arquivo MTL.
- `tests/pbr_surface_maps.rs`: distingue sRGB vs NonColor, tangentes UV, alteração dos pixels gerados e determinismo.

Limites atuais:
- O BSDF ainda é um Cook–Torrance GGX simplificado, não o Principled BSDF/SVM/OSL original.
- Normal map segue convenção +Y (OpenGL), e não converte mapas DirectX -Y automaticamente.
- Mapas escalares usam a componente R 8-bit e substituem o valor constante MTL na região texturizada.
- Texturas demandam UV nos triângulos importados.
- Sem roughness gloss pipelines invertidos, height bump, alpha/transmission, IOR, thin film, multiple scattering, displacement, GPU ou HDRI.
- Requer testes fotométricos e imagens de regressão antes de considerar produção.

A porcentagem é reportada por volume de código fonte, nunca como equivalência de funcionalidades Cycles.
