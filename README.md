# zhop

A modal, **vim-style** fuzzy **tab switcher** and **floating-pane manager** plugin for [Zellij](https://zellij.dev).

![zhop in action](assets/demo.gif)

`Ctrl+y` opens the floating switcher; `j`/`k` to move, `/` to filter by name,
`Enter` to jump to the tab.

Most Zellij tab pickers (e.g. [room](https://github.com/rvcas/room)) are
type-to-filter: every keystroke goes into the filter, so you can't use bare
`j`/`k` to move. `zhop` solves this with two modes:

| Mode | Keys | Action |
|------|------|--------|
| **NORMAL** (default) | `j` / `k` (or `↓` / `↑`) | move selection |
| | `g` / `G` | jump to first / last |
| | `Tab` | toggle **Tabs** ↔ **Panes** view |
| | `Enter` | act on the highlighted row (see below) |
| | `x` | *(Panes view)* close the highlighted floating pane |
| | `/` or `i` | enter INSERT (filter) mode |
| | `q` / `Esc` / `Ctrl+c` | close |
| **INSERT** (filter) | any text | append to fuzzy filter |
| | `Backspace` | delete one char |
| | `Ctrl+u` | clear filter |
| | `↓` / `↑`, `Ctrl+n` / `Ctrl+k` / `Ctrl+p` | move selection |
| | `Esc` | back to NORMAL (keeps the filter) |
| | `Enter` | act on the highlighted row |

So you `j/k` to fly around, and only press `/` when you want to search by name.

### Two views — `Tab` switches between them

| View | Lists | `Enter` | `x` |
|------|-------|---------|-----|
| **Tabs** (default) | the session's tabs | switch to the tab | — |
| **Panes** | the current tab's floating panes | focus the pane | close the pane |

The filter and `j`/`k`/`g`/`G` navigation work the same in both views. So zhop is
both a tab switcher and a quick way to jump between or clean up floating panes.

## Configuration

Passed via the keybinding block (all optional):

| Key | Default | Description |
|-----|---------|-------------|
| `ignore_case` | `true` | case-insensitive filtering |
| `start_in_insert` | `false` | open directly in filter mode |
| `selection_color` | `yellow` | accent color for the highlighted row (ANSI renderer only) |
| `ui` | `ansi` | renderer: `ansi` (self-drawn, fixed palette) or `native` (Zellij UI components, follows the active theme) |

### Renderers

- `ui "ansi"` (default) — draws rows with raw ANSI via owo-colors. Full control,
  but colors are fixed and don't follow your Zellij theme.
- `ui "native"` — uses Zellij's built-in `Text` / `NestedListItem` UI components,
  the same primitives the built-in plugins (session-manager, strider) use. The
  selection highlight and accents automatically match the active theme. The
  `selection_color` option is ignored in this mode.

## Compatibility

Built against `zellij-tile = 0.41`, which targets **Zellij 0.43.x** (wasmtime
runtime). Zellij 0.44 switched the WASM runtime to WASMI and bumped the plugin
API to 0.44, so a separate build will be needed for 0.44+.

## Build

```sh
./build.sh            # release build → target/wasm32-wasip1/release/zhop.wasm
```

Requires the `wasm32-wasip1` target: `rustup target add wasm32-wasip1`.

## Install

### Option A — load directly from a release (no build)

Zellij can load a plugin straight from an HTTPS URL and caches it locally. Pin a
version by pointing at a release asset (replace `v0.1.0` with the tag you want):

```kdl
shared_except "locked" {
    bind "Ctrl y" {
        LaunchOrFocusPlugin "https://github.com/gkstmdgus/zhop/releases/download/v0.1.0/zhop.wasm" {
            floating true
            ignore_case true
            // ui "native"   // uncomment to use the theme-aware renderer
        }
    }
}
```

### Option B — build locally

```sh
./build.sh
cp target/wasm32-wasip1/release/zhop.wasm ~/.config/zellij/plugins/zhop.wasm
```

Then bind it (in `~/.config/zellij/config.kdl`, inside `keybinds`):

```kdl
shared_except "locked" {
    bind "Ctrl y" {
        LaunchOrFocusPlugin "file:~/.config/zellij/plugins/zhop.wasm" {
            floating true
            ignore_case true
            // ui "native"   // uncomment to use the theme-aware renderer
        }
    }
}
```

## License

MIT
