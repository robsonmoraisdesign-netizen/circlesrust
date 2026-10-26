# CirclesRust — conversão gradual do Cycles para Rust

Fonte original usada: [clonecircles](https://github.com/robsonmoraisdesign-netizen/clonecircles), branch main, commit 3eb35f85996baa6c1b2e19bf297b22745c85e321. Tradução modular: o código C++ original continua disponível em outro repositório.

## Já existe código Rust funcional

| Referência original | Arquivo Rust | Entrega |
| --- | --- | --- |
| src/util/hash.h | src/util/hash.rs | Hashes Jenkins e PCG |
| src/util/math_float3.h | src/util/math.rs | Vetores Vec3 em f32 |
| src/util/math_intersect.h e src/kernel/geom/triangle_intersect.h | src/kernel/geometry.rs | Interseção raio-triângulo estilo Pluecker/Embree |
| src/util/boundbox.h | src/bvh/mod.rs | Bounding box AABB e ray slab test |
| src/bvh/build.cpp e src/bvh/bvh2.cpp | src/bvh/mod.rs | BVH binária, divisão por mediana, hits próximos e sombras |
| src/kernel/types.h | src/kernel/types.rs | Flags e configurações do kernel |

Os testes cobrem coordenadas baricêntricas, faces traseiras, limites de distância, raios paralelos, geometria degenerada, identificação de primitivas e comparação BVH contra teste exaustivo.

## Executável de diagnóstico BMP

No terminal, com Rust instalado:

~~~bash
cargo run --release --bin circlesrust-preview -- minha_previa.bmp 960 540
~~~

Isso gera um BMP visível no Windows com dois cubos, piso, teste de raios primários, sombras e luz difusa simplificada. Para testes rápidos:

~~~bash
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
~~~

O GitHub Actions compila Windows/Linux e publica o executável Windows na aba de artefatos da execução, quando aprovada.

**Importante:** o programa BMP é diagnóstico de CPU/BVH; não é o renderizador Cycles completo, não é PBR, não oferece path tracing nem renderização GPU. A interface HTML continua separada do programa Rust nesta etapa.

## Layout

A prévia do [RV Studio Render HTML corrigida](ui/RV_Studio_Render_Layout_Corrigido.html) preserva o layout aprovado, temas claro/escuro e o design system com tokens centralizados.

## Próxima migração

1. Implementar o construtor Cycles BVH2 com SAH/binning e divisão espacial, preservar identificação dos objetos.
2. Implementar câmera e buffers no Rust com integração à UI.
3. Migrar o integrador de caminhos, materiais, luzes e passes, então integrar os dispositivos GPU originais.
4. Empacotar motor e interface como aplicativo de desktop.

Os módulos portados conservam referências de origem e atribuição Apache-2.0.
