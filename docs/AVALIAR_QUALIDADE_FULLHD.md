# Avaliar qualidade e resolução do render

## Por que a imagem anterior era minúscula?

As imagens que o GitHub Actions incluía no ZIP eram **smoke tests**, com `128 × 80` pixels (PNG, 8 amostras) e `48 × 32` pixels (BMP, 2 amostras). Esses arquivos verificavam a exportação, mas não permitiam avaliar acabamento, reflexos, aliasing ou ruído.

A partir desta alteração, o artefato `circlesrust-pathtracer-windows` inclui:

- `avaliacao_fullhd_1920x1080_64spp.png`: **Full HD 1920 × 1080**, com **64 amostras por pixel**. Esta é a imagem de avaliação.
- `preview.png` e `preview.bmp`: arquivos minúsculos destinados exclusivamente aos testes automáticos.
- `circlesrust-preview.exe`: executável Windows em Rust para gerar qualquer resolução até 4096 × 4096.
- `RENDER_QUALIDADE_WINDOWS.bat`: executa o próprio render em Full HD e 128 amostras, sem precisar digitar argumentos.

## Comandos para testar no computador

No mesmo diretório do EXE:

~~~bat
circlesrust-preview.exe Minha_Avaliacao.png 1920 1080 128
~~~

Para uma referência com ainda menos ruído:

~~~bat
circlesrust-preview.exe Minha_Avaliacao_256.png 1920 1080 256
~~~

Resolução 4K (geração demorada e consumo maior de memória):

~~~bat
circlesrust-preview.exe Minha_Avaliacao_4K.png 3840 2160 64
~~~

### Qualidade visual

Mais amostras ajudam a reduzir ruído, mas não acrescentam funcionalidades de Cycles ausentes (shader graph, texturas, HDRI, MIS completo, denoiser, gerenciamento OCIO/AgX, dispositivos GPU). O motor atual é um integrador CPU com GGX simplificado e uma cena fixa.

**GPU:** o código atual do executável `src/bin/circlesrust-preview.rs` usa `render_rgb`, que utiliza `std::thread`; **não há backend GPU** neste repositório Rust. A GPU existente no código C++ do Cycles original em `clonecircles` não é utilizada automaticamente pelo executável Rust.

### Como avaliar corretamente

Abra primeiro `avaliacao_fullhd_1920x1080_64spp.png` no ZIP do workflow mais recente. Compare em ampliação de 100%. Se o limite for serrilhado/ruído, aumente samples. Se a diferença for material, luz ou geometria, será necessário portar mais módulos de Cycles.
