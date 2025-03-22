# Highlog

Patrick Wagstrom &lt;patrick@wagstrom.net&gt;

March 2025

Note: this project is almost entirely vibe coded after I got annoyed with lnav.

Highlog is a terminal user interface application for viewing logs. It uses [ratatui](https://github.com/ratatui-org/ratatui) for rendering and supports both keyboard and mouse input for scrolling through logs.

## Features

- Displays logs in a scrollable window.
- Scroll using keyboard input (Up/Down, PageUp/PageDown).
- Scroll using mouse wheel events (scroll up/down).

## Usage

Build and run the application with Cargo. Make sure to provide the required arguments as per the application's help.

```bash
cargo run -- <CMD>...
```

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
