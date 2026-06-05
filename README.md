# zhop

A modal, **vim-style** fuzzy **tab switcher** plugin for [Zellij](https://zellij.dev).

![zhop in action](assets/demo.gif?v=2)

`Ctrl+y` opens a floating switcher where you `j`/`k` to move, `/` to filter by
name, type an index number to jump straight to a tab, and `Enter` to switch.
Tabs that share a `category:` prefix are grouped under collapsible headers, and
`Tab` toggles between the grouped and flat views.

- **Modal, vim-style** — `j`/`k` move without typing into a filter; `/` only when you want to search.
- **Category grouping** — name tabs `dev:server`, `web:docs`, … and zhop buckets them under `dev` / `web` headers (prefix stripped). `Tab` flattens it back.
- **Index jump** — every row shows its tab number; type it and press `Enter` to go straight there.
- **Theme-aware** — drawn with Zellij's native UI, so colors follow your active theme.

Most Zellij tab pickers (e.g. [room](https://github.com/rvcas/room)) are
type-to-filter: every keystroke goes into the filter, so you can't use bare
`j`/`k` to move. `zhop` solves this with two modes:

| Mode | Keys | Action |
|------|------|--------|
| **NORMAL** (default) | `j` / `k` (or `↓` / `↑`) | move selection |
| | `g` / `G` | jump to first / last |
| | `0`–`9` | type a tab's index number to select it (then `Enter` to jump) |
| | `Tab` | toggle grouped / flat view |
| | `Enter` | switch to the highlighted tab |
| | `/` or `i` | enter INSERT (filter) mode |
| | `q` / `Esc` / `Ctrl+c` | close |
| **INSERT** (filter) | any text | append to fuzzy filter |
| | `Backspace` | delete one char |
| | `Ctrl+u` | clear filter |
| | `↓` / `↑`, `Ctrl+n` / `Ctrl+k` / `Ctrl+p` | move selection |
| | `Tab` | toggle grouped / flat view |
| | `Esc` | back to NORMAL (keeps the filter) |
| | `Enter` | switch to the highlighted tab |

So you `j/k` to fly around, and only press `/` when you want to search by name.

Each row shows the tab's index number; in NORMAL mode type it to select that tab,
then press `Enter` to jump there. Digits accumulate (e.g. `1` then `2` → tab 12), so
the number is never ambiguous. The pending number shows as `#…` in the header;
`Backspace` edits it and `Esc` cancels it. Pressing `Enter` on an index that doesn't
exist shows a brief warning instead of jumping.

## Configuration

Passed via the keybinding block (all optional):

| Key | Default | Description |
|-----|---------|-------------|
| `ignore_case` | `true` | case-insensitive filtering |
| `start_in_insert` | `false` | open directly in filter mode |
| `group_by_prefix` | `true` | group tabs by a category prefix (see [Grouping](#grouping)) |
| `group_delimiter` | `:` | separator between the category prefix and the tab name |

The switcher renders with Zellij's native UI components, so the selection
highlight and accent colors follow your active theme automatically.

### Grouping

When `group_by_prefix` is on (the default), a tab named like `category<delimiter>name`
is shown under a category header with the prefix stripped from the row. For example,
with the default `:` delimiter, the tabs:

```
work:server   work:db   web:docs   scratch
```

render as:

```
work
   server
   db
web
   docs
scratch
```

Tabs without a usable prefix (no delimiter, or nothing on either side of it) stay
ungrouped. Filtering still matches against the full tab name, and `j`/`k` navigation
follows the on-screen order.

Press `Tab` inside the switcher to toggle between the grouped and flat views on the
fly. `group_by_prefix` only sets the initial state; set it to `false` to open flat.
Change `group_delimiter` (e.g. `"/"`) to group on a different separator.

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
        }
    }
}
```

## License

MIT
