# Etapa de geometria CPU e BVH2

Referência fixa: [clonecircles main@3eb35f8](https://github.com/robsonmoraisdesign-netizen/clonecircles/tree/3eb35f85996baa6c1b2e19bf297b22745c85e321)

## Fonte e abrangência

- src/util/math_float3.h e src/util/boundbox.h: Vec3, caixas alinhadas e união de limites;
- src/util/math_intersect.h: interseção raio-triângulo por coordenadas de Pluecker, verificações de borda e profundidade;
- src/kernel/geom/triangle_intersect.h: propagação da interseção e baricêntricas;
- src/bvh/build.cpp, src/bvh/bvh2.cpp, src/bvh/params.h: construção de árvore binária, até 8 triângulos por folha; distância de primeiro/mais próximo acerto.

A BVH desta etapa realiza uma divisão pela mediana do centroide, sem custo SAH. O algoritmo não é uma tradução completa da estrutura do Cycles, que inclui outras primitivas, SAH, motion blur, objetos instanciados, hierarquia GPU e interseções de curvas.

## Invariantes e testes

- Raios com origem e direção finitas, intervalo não negativo e máximo maior que mínimo;
- Triângulos inválidos não entram na hierarquia, mas o índice original é preservado;
- Interseções por trás da câmera ou além do limite t_max são rejeitadas;
- Testes para direções paralelas a faces AABB evitam o problema numérico de 0 multiplicado por infinito;
- Travessia por pilha e visita primeiro do nó mais próximo;
- Comparação de centenas de raios com busca exaustiva;
- Arquivo BMP de teste gerado diretamente de interseções Rust com sombras binárias.

Sem afirmar equivalência bit a bit com Cycles/Embree: otimizações SIMD, precisão de ponto flutuante e cenas complexas exigem validação de referência separada.
