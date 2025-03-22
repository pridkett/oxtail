# Highlog

Patrick Wagstrom &lt;patrick@wagstrom.net&gt;

March 2025

Note: this project is almost entirely vibe coded after I got annoyed with lnav.

Highlog is a terminal user interface application for viewing logs. It uses [ratatui](https://github.com/ratatui-org/ratatui) for rendering and supports both keyboard and mouse input for scrolling through logs.

## Features

- Displays logs in a scrollable window.
- Scroll using keyboard input (Up/Down, PageUp/PageDown).
- Scroll using mouse wheel events (scroll up/down).
- Command mode for filtering and customizing the display.

## Command Mode

Press `:` to enter command mode, where you can type commands to modify the display:

- `:show_source stdout/stderr/all` - Show logs from the specified source.
- `:hide_source stdout/stderr/all` - Hide logs from the specified source.
- `:show_meta time/source/lines` - Show the specified metadata.
- `:hide_meta time/source/lines` - Hide the specified metadata.

Command mode features include:
- Command history navigation with up/down arrow keys
- Text editing with left/right arrow keys and cursor positioning
- Readline-like bindings:
  - Ctrl+A: Move to start of line
  - Ctrl+E: Move to end of line
  - Ctrl+K: Kill (delete) to end of line
  - Ctrl+U: Kill to beginning of line
  - Ctrl+W: Delete word backward
  - Ctrl+R: Reverse search through command history

## Usage

Build and run the application with Cargo. Make sure to provide the required arguments as per the application's help.

```bash
cargo run -- <CMD>...
```

## Keyboard Controls

- `q` - Quit the application
- `:` - Enter command mode
- `ESC` - Exit command mode
- Up/Down arrows - Scroll one line up/down
- PageUp/PageDown - Scroll one page up/down
- Mouse wheel - Scroll up/down

## Setup

Install dependencies and build the project with:

```bash
cargo build
```

Then, run the application with:

```bash
cargo run -- <CMD>...
```

## License

MIT License
