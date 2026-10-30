# Regras permanentes do CirclesRust

- Toda migração e toda função de produto (render, geometria, comandos, automações, importação e interface desktop) devem ser implementadas em Rust.
- Não usar JavaScript/TypeScript, Node, WebView ou scripts HTML para substituir partes funcionais do software. Não incorporar o motor C++ como se fosse uma migração Rust.
- O HTML já existente em `ui/` é somente referência visual arquivada; preservar a referência, sem ligá-la ao aplicativo. Shaders GPU futuros são código do backend, não justificativa para um runtime JavaScript.
- Toda entrega deve informar o percentual de acompanhamento, a métrica, a base de referência e suas limitações. Não chamar razão de linhas de código de percentual funcional do Cycles.
- Atualizar `docs/PROGRESSO_MIGRACAO.md` com os números, recursos entregues, testes e pendências. Usar o contador Rust `migration-progress` para reproduzir o indicador de volume.
- Alterações devem estar no fonte e no Git. Entregar o executável Windows e o código correspondente em um ZIP quando a compilação estiver disponível.
- Testar o núcleo e verificar compilação desktop; não declarar interação visual Windows ou aceleração GPU validada apenas porque compilou. Manter aparência centralizada em `src/desktop/theme.rs`.
