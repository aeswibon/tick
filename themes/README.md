# tick theme gallery

Built-in themes are compiled into the binary. Copy any file here to `~/.config/tick/themes/` and edit to customize.

| Theme | File | Notes |
|-------|------|-------|
| default | `default.toml` | Catppuccin-style dark (default) |
| light | `light.toml` | Light background |
| tokyo-night | `tokyo-night.toml` | Tokyo Night |
| dracula | `dracula.toml` | Dracula |

```bash
tick --list-themes
```

Set in `config.toml`:

```toml
theme = "dracula"
```

Or at launch: `tick --theme tokyo-night`
