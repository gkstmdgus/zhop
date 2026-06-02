# Contributing to zhop

Issues and pull requests are welcome.

## Reporting issues

Open an issue with the bug-report or feature-request template. For bugs, please
include your **Zellij version** — zhop is built against `zellij-tile 0.41`
(Zellij 0.43.x), and the plugin API differs on 0.44+.

## Pull requests

1. Fork the repo and create a branch off `main`.
2. Make your change. Keep commits focused — one logical change per commit.
3. Build and test:
   ```sh
   ./build.sh
   cp target/wasm32-wasip1/release/zhop.wasm ~/.config/zellij/plugins/zhop.wasm
   # then reload in a running session:
   zellij action start-or-reload-plugin "file:$HOME/.config/zellij/plugins/zhop.wasm"
   ```
4. Open a PR against `main` and fill in the template.

`main` is protected, so changes land through PRs. CI builds every push and PR; a
green build is required before merge.

## Code style

Match the surrounding code — naming, comment density, and idiom. The whole
plugin lives in `src/main.rs`.

## License

By contributing you agree your contributions are licensed under the [MIT License](LICENSE).
