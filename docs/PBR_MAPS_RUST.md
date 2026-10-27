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


## Panoramas de ambiente (etapa adicional)

O programa aceita --environment arquivo.hdr, .png ou .jpg e --env-strength 0..1000. O HDR Radiance RGBE é convertido para valores RGB float, permitindo luz acima de 1. O método sample_latlong deriva U pela longitude atan2 e V pela latitude acos; U repete e V é limitado nos polos. Texturas LDR usam transferência sRGB, enquanto Radiance HDR é linear.

A iluminação por ambiente é avaliada para todos os raios que não atingem geometria e no estágio de iluminação direta ambiente. A amostragem não possui tabelas de importância do HDRI e não representa o integrador completo do Cycles.

Os testes em tests/environment_map.rs verificam persistência de luminância HDR superior a 1, coordenadas nos polos, alteração efetiva de pixels e determinismo, sem confundir o simples carregamento do mapa com uso no render.
