# TerranoxOS Fonts

## Required Fonts

| Font | Usage | Source |
|------|-------|--------|
| **Inter** | UI text (labels, buttons, menus) | https://rsms.me/inter/ |
| **JetBrains Mono** | Monospace (terminal, code, logs) | https://www.jetbrains.com/lp/mono/ |

## Installation

```bash
# Arch Linux
sudo pacman -S inter-font ttf-jetbrains-mono

# Fedora
sudo dnf install google-noto-sans-fonts jetbrains-mono-fonts

# Ubuntu
sudo apt install fonts-inter fonts-jetbrains-mono
```

## GTK4 Font Configuration

Set in `gtk4.css` or via `GSettings`:

```css
* {
    font-family: "Inter", sans-serif;
    font-size: 14px;
}

.monospace {
    font-family: "JetBrains Mono", monospace;
    font-size: 13px;
}
```

## Cursor

**Bibata-Modern-Classic** — recommended cursor theme.

```bash
# Install
yay -S bibata-cursor-theme
# Or download from: https://github.com/ful1e5/Bibata_Cursor

# Set in Hyprland
env = XCURSOR_THEME,Bibata-Modern-Classic
env = XCURSOR_SIZE,24
```

## Icons

**Papirus Dark** — recolored with violet (#8b5cf6) accent for Obsidian, crimson (#ef4444) for Sentinel.

```bash
sudo pacman -S papirus-icon-theme
# Then use papirus-folders to set accent color:
papirus-folders -C violet
```
