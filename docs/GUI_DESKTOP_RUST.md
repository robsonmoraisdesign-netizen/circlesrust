# RV Studio Render — primeira interface desktop nativa Rust

## Como usar no Windows

Na aba GitHub **Actions**, selecione a última execução bem-sucedida e baixe o pacote `RV-Studio-CirclesRust-Windows-Desktop`. Extraia e inicie `circlesrust-studio.exe` por duplo clique.

A interface não exige que o usuário localize Blender, LuxCore ou outro motor externo. O renderizador CPU atual está vinculado diretamente ao mesmo processo do aplicativo.

## Layout

Baseado na referência `ui/RV_Studio_Render_Layout_Corrigido.html`:
- Barra superior com `RENDERIZAR`, resolução, amostras e tema;
- Esquerda: ambiente HDR, luz e outliner/importação OBJ/MTL;
- Centro: imagem real calculada (preview após render), configuração do arquivo de saída, amostras/ressaltos;
- Direita: propriedades de materiais (cor, metalicidade, rugosidade);
- Inferior: posição da câmera, alvo, FOV e exposição;
- Status inferior mostra andamento e o caminho da imagem produzida.

A identidade visual usa `src/desktop/theme.rs` como o **módulo central de aparência**; elementos da UI não definem suas próprias cores. O tema claro/escuro é selecionável.

## Caminho de renderização real

1. O usuário informa uma malha OBJ, opcionalmente MTL, e um HDR.
2. `src/scene/loader.rs` importa geometria, texturas, mapas PBR e configura o ambiente.
3. Parâmetros de materiais, luz e câmera alterados na interface são aplicados ao `Scene` Rust.
4. O botão Renderizar executa `render_rgb_denoised_with_camera` em uma thread de CPU.
5. A aplicação exibe os pixels RGB devolvidos pelo motor e escreve PNG ou BMP no caminho informado.

## Compilação

~~~powershell
cargo build --release --features desktop-ui --bin circlesrust-studio
.\target\release\circlesrust-studio.exe
~~~

A ferramenta CLI antiga continua disponível:

~~~powershell
cargo build --release --bin circlesrust-preview
~~~

## Limitações desta etapa

- **A região central ainda é uma prévia de pixels renderizados**, não um viewport 3D de modelagem em tempo real com seleção e transformações.
- Renderização via **CPU**: ainda não existe dispositivo GPU Cycles portado/integrado ao executável Rust.
- Não há gerenciador de projetos nem editor de nós visual; materiais importados do MTL podem conter texturas PBR, e alterações de cor base em material com mapa podem precisar recarregar o material para recomputar o nó de textura.
- O caminho para OBJ/HDR/MTL é informado em campos de texto (a janela ainda não tem file picker).
- A aplicação tem uma única cena carregada de cada vez; o outliner representa os materiais de cena e o modelo importado, não uma árvore de objetos paramétrica.
- Progressão durante render: exibe estado ocupado, não porcentagem por amostra. O cálculo ocorre em uma thread de CPU de fundo enquanto a interface continua responsiva.
- A aparência segue a divisão aprovada, mas **não é uma réplica pixel a pixel** do HTML de referência.

O software está sendo implementado em Rust. O original C++ do Cycles serve como referência, **não como binário compilado** neste projeto.
