# tick theme gallery

Built-in themes are compiled into the binary. Copy any file here to `~/.config/tick/themes/` and edit to customize.

| Theme | File | Notes |
|-------|------|-------|
| default | `default.toml` | Catppuccin Mocha-style dark (default) |
| catppuccin-mocha | `catppuccin-mocha.toml` | Same as default |
| light | `light.toml` | Light background |
| tokyo-night | `tokyo-night.toml` | Tokyo Night |
| dracula | `dracula.toml` | Dracula |
| gruvbox-dark | `gruvbox-dark.toml` | Gruvbox dark |
| nord | `nord.toml` | Nord |
| one-dark | `one-dark.toml` | Atom One Dark |
| solarized-dark | `solarized-dark.toml` | Solarized Dark |
| rose-pine | `rose-pine.toml` | Rose Pine Moon |

```bash
tick --list-themes
```

Set in `config.toml`:

```toml
theme = "dracula"
```

Or at launch: `tick --theme tokyo-night`
