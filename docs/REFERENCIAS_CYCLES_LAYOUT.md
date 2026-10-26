# Referências de interface e motor de renderização

## Código de origem do motor

- Fonte fornecida pelo usuário: [clonecircles](https://github.com/robsonmoraisdesign-netizen/clonecircles)
- Branch `main`, commit `3eb35f85996baa6c1b2e19bf297b22745c85e321`, com a árvore original do Cycles.
- Módulos identificados: `src/kernel`, `src/integrator`, `src/scene`, `src/device`, `src/bvh` e `src/util`.
- Projeto de migração: `circlesrust`, com uma primeira biblioteca parcial Rust; o código C++ do clone não é removido ou sobrescrito.

## Referência da interface

- Base fornecida: `RV_Studio_Render_Layout_Base(1).html`.
- Versão corrigida neste repositório: [ui/RV_Studio_Render_Layout_Corrigido.html](../ui/RV_Studio_Render_Layout_Corrigido.html).
- A composição conserva ambiente/global à esquerda, outliner abaixo, viewport e câmera no centro, materiais à direita, cabeçalho superior e temas claro/escuro.
- Correções: responsividade sem largura mínima fixa, cabeçalho de duas linhas em janelas médias, viewport mais legível, propriedades com rolagem interna, botões de materiais sem truncamento, ícone da luz visível, prévia de cor atualizada, popovers na área visível e teclas de navegação.
- Verificações manuais automatizadas em Chromium local: 1920×1080, 1440×900, 1280×720, 1024×768, 900×650, 760×760, 620×760 e 375×812. Sem erros JavaScript nos fluxos testados.

## Limitações de implementação

O HTML demonstra o layout e o viewport ilustrativo em Canvas 2D; **não executa Cycles nem LuxCore**. Render GPU, PBR fisicamente correto, denoise e importação/visualização de geometria complexa dependem da integração futura do motor. Não apresentar a prévia como uma renderização de produção.

A aparência de interface é controlada pelas variáveis `--rv-*` centralizadas; cores de materiais são dados de cena, não aparência de componentes.
