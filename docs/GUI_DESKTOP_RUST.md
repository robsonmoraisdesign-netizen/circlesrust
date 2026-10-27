# RV Studio Render — pré-visualização e layout responsivo em Rust

## Principais correções

**Ambiente 3D**: a janela inicia mostrando geometria real da BVH (OBJ ou cena procedural), colorida por material e iluminada rapidamente. Não exige executar o render final. Clique e arraste com o botão esquerdo para orbitar; role o mouse sobre a visualização para ampliar/reduzir.

**Tempo real**: pré-visualização de path tracing **CPU**, atualizada automaticamente após mudanças na câmera, materiais ou luzes. Faz três etapas de refinamento com 1, 3 e 8 amostras por pixel em resolução reduzida. Isto **não é um viewport GPU nem taxas de quadros de jogos**. Quando há mudança durante um cálculo, o resultado obsoleto é descartado e o cálculo atualizado entra na sequência.

**Imagem final**: o comando RENDERIZAR preserva a imagem final em resolução configurada e salva PNG/BMP. Enquanto o render está em andamento, pode clicar em **Voltar ao ambiente** para visualizar e ajustar a cena. A imagem final permanece disponível em sua aba quando concluída.

**Painéis**: larguras laterais são limitadas conforme o tamanho atual da janela; altura do painel de câmera tem limites; controles usam disposição com quebra de linha e scroll vertical nas laterais. O viewport mantém a proporção da imagem ao redimensionar, sem cortar ou esticar.

**Temas**: cores e dimensões são controladas exclusivamente em `src/desktop/theme.rs`.

## Instalação

Na última execução bem-sucedida em Actions, baixar artifact **RV-Studio-CirclesRust-Windows-Desktop** e executar `circlesrust-studio.exe`.

~~~powershell
cargo build --release --features desktop-ui --bin circlesrust-studio
~~~

O aplicativo integra o renderizador Rust no mesmo processo, sem depender de pasta externa do Blender ou LuxCore.

## Limitações honestas

- Modo Ambiente: **CPU raycast** pela BVH existente, não rasterização por GPU.
- Modo Tempo real: **path tracing CPU progressivo em baixa resolução**, não atualização a 30/60 FPS nem render GPU acelerado. A cada refinamento o atual integrador reexecuta a amostragem, sem reutilizar accumulation buffers.
- Modelo e iluminação continuam com as limitações de shading da migração parcial do Cycles.
- A pré-visualização por material mostra faces trianguladas importadas, mas a lista lateral ainda usa materiais/arquivo como agrupadores; não existe ainda seleção de objeto diretamente na malha nem gizmos de transformação.
- Durante um render completo, a imagem é gravada a partir de um snapshot da cena. É possível voltar ao ambiente enquanto processa; mudanças posteriores não alteram o render que já iniciou.
- CPU e C++: todo código executável desenvolvido até aqui é Rust; o Cycles C++ é referência e não está integrado.
