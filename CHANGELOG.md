# Changelog

## [0.3.1](https://github.com/pataruco/zed-mjml/compare/zed-mjml-v0.3.0...zed-mjml-v0.3.1) (2026-10-04)


### Bug Fixes

* **lsp:** survive panics on multi-byte snippets and harden the deploy workflow ([b336039](https://github.com/pataruco/zed-mjml/commit/b336039336fae667209d0f484cd629fc30795ef1))
* **lsp:** truncate error snippets on a character boundary and survive panics ([5110a41](https://github.com/pataruco/zed-mjml/commit/5110a414392a053212ec996a8767db8e6eeffad8))

## [0.3.0](https://github.com/pataruco/zed-mjml/compare/zed-mjml-v0.2.0...zed-mjml-v0.3.0) (2026-10-04)


### Features

* **lsp:** resolve mj-include paths relative to the document ([48cc89e](https://github.com/pataruco/zed-mjml/commit/48cc89e237df501d6e6227b692bfca867f613320))


### Bug Fixes

* false errors in nesting validation ([b32e939](https://github.com/pataruco/zed-mjml/commit/b32e939d756c125ea1154c1b4fb6373b3db7bf8f))
* **lsp:** allow any component inside mj-attributes and treat rootless files as partials ([bc7841e](https://github.com/pataruco/zed-mjml/commit/bc7841eb3252d2ee9826d5f497f66f7c11070e56))
* **lsp:** bump lsp-server to 0.10 and refresh lock file ([e7f4c04](https://github.com/pataruco/zed-mjml/commit/e7f4c0470e733c38f3f19b08eed59d2ab0ee5dea))
* **lsp:** bump lsp-server to 0.10 and refresh lock file ([e87f6af](https://github.com/pataruco/zed-mjml/commit/e87f6afcddc8e70567902965515f01cee0806490))
* **lsp:** do not report mrml's inline-style build warning ([ffbd730](https://github.com/pataruco/zed-mjml/commit/ffbd730093925b3545da493b94dce5865d1867ee))

## [0.2.0](https://github.com/pataruco/zed-mjml/compare/zed-mjml-v0.1.0...zed-mjml-v0.2.0) (2026-06-16)


### Features

* add completion support ([c9ef3c0](https://github.com/pataruco/zed-mjml/commit/c9ef3c0adbbfc3af4f12b9f83256a43bcbf592d1))
* add completion support ([67948e1](https://github.com/pataruco/zed-mjml/commit/67948e1df3dde06d3f3cb46750c6dacbc2a9b0eb))
* add quick fixes for unknown tags and missing attributes ([56668ca](https://github.com/pataruco/zed-mjml/commit/56668cab5d1c07098d6337a2105167afed046e4a))
* add snippets ([351373f](https://github.com/pataruco/zed-mjml/commit/351373fc495a0f2f966f6d3cb7718290187e36f3))
* complete all mjml attribute types ([8f61adb](https://github.com/pataruco/zed-mjml/commit/8f61adb06108a693862a26f96141623da119aa07))
* complete all mjml attribute types ([e62373f](https://github.com/pataruco/zed-mjml/commit/e62373fc59c2fda4ab77a9fdcd4ee7dc4794a039))
* hover and completion ([19d8011](https://github.com/pataruco/zed-mjml/commit/19d801122a72550add8f2d537097bf1ee0af1504))
* hover and completion ([ffbad01](https://github.com/pataruco/zed-mjml/commit/ffbad019fa59b774b6566d5088942d936cef0150))


### Bug Fixes

* managing multiple versions ([f9085c2](https://github.com/pataruco/zed-mjml/commit/f9085c2027746f4990f359ce321bff84c9bdec05))

## 0.1.0 (2026-03-07)


### Features

* add mjml language support ([a60e50d](https://github.com/pataruco/zed-mjml/commit/a60e50d295be59770e96a19c06ae6df416f5257a))
* add mjml-lsp binary crate with validation and diagnostic publishing ([ded94e4](https://github.com/pataruco/zed-mjml/commit/ded94e4afcf20f2c546e612e6eabf0a7b511f704))
* add Zed extension WASM entry point with LSP download ([beb3c9e](https://github.com/pataruco/zed-mjml/commit/beb3c9edbb93ebb09f4bb24e92d6dca3c1ef652c))
* all validation rules ([e30e4c2](https://github.com/pataruco/zed-mjml/commit/e30e4c241aef61af44a9aeb17129bcb6ac42c503))
* cover all mjml validation rules ([c91b6e1](https://github.com/pataruco/zed-mjml/commit/c91b6e1b6c85052fe90152825b01a95d3738d3c2))
* deploy ([257e6e1](https://github.com/pataruco/zed-mjml/commit/257e6e1d2961fed2928d920832594ba5a3dec177))
* refactor highlights ([13ed83f](https://github.com/pataruco/zed-mjml/commit/13ed83ffa327dcc150287ae7141ec9cbefd880ec))
* set useful error messages ([f74690d](https://github.com/pataruco/zed-mjml/commit/f74690d49b0cae9526796259cf01922a5d0cba99))
