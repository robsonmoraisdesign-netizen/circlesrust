# RV Studio Render — pré-visualização e layout responsivo em Rust

## Principais correções

**Ambiente 3D**: a janela inicia desenhando as malhas reais por rasterização wgpu (OBJ ou cena procedural), colorida por material e iluminada rapidamente. Não exige executar o render final. Clique e arraste com o botão esquerdo para orbitar; role o mouse sobre a visualização para ampliar/reduzir.

**Tempo real**: pré-visualização de path tracing **CPU**, atualizada automaticamente após mudanças na câmera, materiais ou luzes. Faz três etapas de refinamento com 1, 2 e 4 amostras por pixel em resolução reduzida. Durante a interação, mostra a prévia sólida wgpu; o refinamento continua CPU. Quando há mudança durante um cálculo, o resultado obsoleto é descartado e o cálculo atualizado entra na sequência.

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

- Modo Ambiente: **rasterização wgpu** com malha persistente, profundidade e transformação por matrizes; cor base e iluminação simples.
- Modo Tempo real: **path tracing CPU progressivo em baixa resolução**, não atualização a 30/60 FPS nem render GPU acelerado. A cada refinamento o atual integrador reexecuta a amostragem, sem reutilizar accumulation buffers.
- Modelo e iluminação continuam com as limitações de shading da migração parcial do Cycles.
- Seleção de objetos e handles mover/escalar estão implementados. Anéis de rotação, snapping e instâncias ainda faltam.
- Durante um render completo, a imagem é gravada a partir de um snapshot da cena. É possível voltar ao ambiente enquanto processa; mudanças posteriores não alteram o render que já iniciou.
- CPU e C++: todo código executável desenvolvido até aqui é Rust; o Cycles C++ é referência e não está integrado.


## Navegação aprimorada e resposta da prévia

- **Botão do meio do mouse (MMB):** orbita a câmera, seguindo o padrão Blender.
- **Shift + botão do meio:** pan. Origem e alvo deslocam-se juntos, preservando a orientação da câmera; a velocidade depende da distância do objeto e do tamanho do viewport.
- **Botão esquerdo:** arrastar com a ferramenta ativa, **Orbitar** ou **Pan**, selecionável na barra acima da visualização.
- **Roda do mouse:** aproxima e afasta a câmera.
- **Redefinir câmera:** volta à posição inicial, sem perder o modelo e os materiais.

Os comandos operam nos parâmetros reais da câmera, portanto afetam o viewport sólido, a prévia ao vivo e qualquer novo render final.

### Redução de atrasos na navegação (CPU)

O renderer agora utiliza uma geração atômica compartilhada para identificar imagens obsoletas. Ao mudar a câmera, a luz ou um material, operações CPU do viewport sólido e da prévia ray-traced verificam esse token durante a computação; se a versão da cena já mudou, deixam de usar a CPU para completar o quadro anterior. O render final iniciado pelo usuário continua independente, sem ser cancelado.

As prévias têm debounce de 75 ms (sólida) e 160 ms (tempo real), para não disputar CPU durante cada movimento do mouse. O tamanho padrão do viewport sólido foi reduzido de 680 para 580 pixels de largura e a prévia ray-traced de 420 para 360; as amostras progressivas foram reduzidas de 1→3→8 para 1→2→4. Isso aumenta a responsividade ao custo de resolução/qualidade **apenas da pré-visualização**, sem alterar a resolução, qualidade nem amostras escolhidas para o render final.

A seção acima descreve o fallback CPU. O modo padrão agora usa wgpu, sem debounce para navegar no Ambiente. O custo ainda depende da malha e do adaptador.


## Seleção por geometria e F para enquadrar

O motor armazena triangle_objects e object_names separados de triangle_materials. No OBJ, comandos o/g definem as regiões reconhecidas como objetos/grupos; na cena de estúdio, os modelos procedurais recebem IDs definidos na montagem. Ao clicar dentro dos pixels realmente exibidos (respeitando a área com letterbox), o aplicativo lança um raio correspondente à câmera e seleciona o triângulo mais próximo usando a BVH original. A seleção pode ser alterada pelo painel Objetos.

A aba Ambiente 3D destaca a região selecionada em âmbar, sem modificar a cor ou os materiais utilizados pelo render final. O atalho F ou botão Enquadrar selecionado reposiciona a câmera para o bounding box da geometria selecionada. As abas e os controles Pan/Órbita/Zoom permanecem independentes da seleção.

O HTML não é executado nem compilado pelo programa Rust, e foi preservado sem modificação. O novo .gitattributes configura ui/*.html como linguist-documentation, alterando apenas a classificação de linguagem no GitHub.


## Transformações de objeto em tempo de execução (Rust)

Selecione um objeto ou grupo OBJ; use as propriedades do painel direito **Posição**, **Rotação XYZ (graus)** e **Escala XYZ** para editar valores ou restaurá-los. O modo de viewport agora oferece Mover (G), Girar (R), Escala (S) e seleção de eixo Livre/X/Y/Z. Arrastar com o botão esquerdo aplica a ferramenta atual ao objeto selecionado. Indicadores coloridos X/Y/Z são desenhados na pré-visualização como referência visual, **não ainda manipuladores clicáveis por eixo**. Navegação MMB, Shift+MMB e zoom permanecem.

O novo `src/scene/transform.rs` armazena os parâmetros por objeto, aplica a geometria de origem conservada, recalcula normais de vértice pela transformação inversa-transposta (escala e rotação) e regenera a BVH Rust para que o render final/preview realmente mostre as alterações. O OBJ continua com UV e materiais, preservados após o transform. O render já iniciado usa um snapshot inalterado.

Sem GPU, sem instancing, sem negativo na escala, sem manipuladores interativos de eixos e sem histórico undo/redo persistente. A regeneração de BVH pode exigir otimização adicional em cenas maiores.


## Histórico de transformação (Rust)

Ctrl+Z desfaz, Ctrl+Shift+Z/Ctrl+Y refazem; botões Desfazer/Refazer ficam no painel de transformação. Os atalhos não interceptam campos de texto em edição. Cada arraste G/R/S ou de um campo numérico é agrupado em uma operação; Esc durante o arraste cancela e exige soltar o botão antes de outro gesto. Valores digitados são registrados quando o controle os aplica. Restaurar transformação também pode ser desfeito.

O limite é 128 operações por sessão/cena, armazenadas como comandos pequenos; desfazer reconstrói a BVH a partir da malha original. Não é histórico de câmera, materiais ou importações, nem salvamento de projeto. Render final em andamento preserva seu snapshot.

## Handles X/Y/Z

Em Mover e Escala, arraste a linha ou ponta de um eixo. O destaque indica o alvo; mover usa pontas circulares, escala usa quadradas. Arrastar para fora aumenta a escala e para dentro reduz. O eixo fica travado durante o gesto, com uma operação no histórico e cancelamento por Esc. Eixos quase alinhados à câmera ficam ocultos. Girar mantém o controle anterior, sem anéis nesta versão.

## Viewport SolidCraft adaptado

Execute `circlesrust-studio.exe` para usar wgpu. `ABRIR_MODO_COMPATIBILIDADE.bat` usa o viewport CPU. Consulte `SOLIDCRAFT_VIEWPORT.md` para a integração, testes e limites: núcleo CAD completo ainda pendente, render final CPU.
