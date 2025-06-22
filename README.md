# macOS Tweaks

[![Rust](https://img.shields.io/badge/made%20with-Rust-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A terminal-based utility for managing macOS tweaks, built with Rust. It features both an interactive terminal user interface (TUI) and a command-line interface (CLI).

![Screenshot](/screenshots/1.png)

## Features

- **Interactive TUI**: User-friendly terminal UI for browsing tweaks.
- **Command-Line Interface**: For scripting and direct commands.
- **Tweak Library**: Collection of tweaks for Dock, Power Management, Networking, etc.
- **Homebrew Management**: Manage Homebrew packages directly.
- **Customizable Colors**: Adjust the app's appearance via a JSON config file.
- **Safe and Reversible**: Most tweaks can be easily reverted.

## Installation

### Prerequisites

- **Rust**: Install the latest stable version.

### From Source

1. **Clone the repository**:
    ```bash
    git clone https://github.com/itsdoruk/macos-tweaks.git
    cd macos-tweaks
    ```

2. **Install the command**:
    ```bash
    cargo install --path .
    ```

Now you can run `macos-tweaks --help` from anywhere.

## Usage

Run the application in either **interactive (TUI)** or **command-line (CLI)** mode.

### Interactive Mode

Launch the TUI with:
```bash
macos-tweaks
```

**Navigation:**
- **↑↓**: Navigate lists.
- **←→**: Move between categories.
- **Enter**: Select or apply a tweak.
- **q**: Quit.
- **Esc**: Go back.

### Command-Line Interface (CLI)

For quick actions, use CLI commands.

#### `list`
Lists all available tweaks:
```bash
macos-tweaks list
```

#### `apply <TWEAK_NAME>`
Applies a specific tweak:
```bash
macos-tweaks apply "Clean Up Homebrew"
```

#### `revert <TWEAK_NAME>`
Reverts a specific tweak:
```bash
macos-tweaks revert "Auto-hide Dock"
```

## Configuration

Customize the app's appearance with a configuration file located at `~/.config/macos-tweaks/config.json`. The default file will be created on the first run.

### Color Scheme Example
```json
{
  "color_scheme": {
    "primary": "#fe640b",
    "secondary": "#ffffff",
    "accent": "#00ff00",
    "success": "#00ff00",
    "warning": "#ffa500",
    "error": "#ff0000",
    "text": "#ffffff",
    "text_dim": "#808080"
  },
  "theme": "default"
}
```

## Available Tweaks

Categories include:
- **Dock**: Customize Dock settings.
- **Animated Wallpapers**: Set video backgrounds.
- **Power Management**: Adjust sleep settings.
- **Networking**: Manage network settings.
- **Optimization**: Clean system caches.
- **Brew Management**: Handle Homebrew packages.
- **About**: View app version and system info.

## Development

### Adding New Tweaks

To add a tweak, modify `src/app.rs` and add a new `Tweak` struct in the `App::new()` function.

```rust
Tweak::new(
    "My New Tweak",
    "Description.",
    "command-to-run",     
    "command-to-revert",  
    false                 
)
```

## Disclaimer

This application modifies system settings. Use at your own risk and back up your data. The author is not responsible for any damage.

## License

Licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
