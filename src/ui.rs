use std::io;
use std::sync::mpsc::Receiver;
use std::time::Duration;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
    widgets::{Block, Borders, Paragraph, Wrap},
    text::{Span, Spans},
    style::{Color, Modifier, Style},
};

pub fn run_ui(rx: Receiver<String>) {
    // Setup terminal
    enable_raw_mode().unwrap();
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    // Vector to store output lines received from process_handler
    let mut output: Vec<String> = Vec::new();
    // Scroll offset: number of lines from the bottom.
    let mut scroll_offset: usize = 0;

    // Application loop
    loop {
        // Non-blocking check for new messages
        while let Ok(line) = rx.try_recv() {
            output.push(line);
        }

        terminal.draw(|f| {
            let size = f.size();
            let block = Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    "Highlog - Neon Terminal UI",
                    Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                ));

            let total_lines = output.len();
            // Calculate visible lines based on terminal height (minus 2 for borders)
            let visible_count = (size.height as usize).saturating_sub(2);
            let start = if total_lines > visible_count + scroll_offset {
                total_lines - visible_count - scroll_offset
            } else {
                0
            };
            let end = total_lines - scroll_offset;
            let display_lines: Vec<Spans> = output[start..end]
                .iter()
                .map(|line| Spans::from(Span::styled(line, Style::default().fg(Color::Yellow))))
                .collect();

            let paragraph = Paragraph::new(display_lines)
                .block(block)
                .wrap(Wrap { trim: true });
            f.render_widget(paragraph, size);
        }).unwrap();

        if event::poll(Duration::from_millis(200)).unwrap() {
            // Calculate visible lines based on current terminal height
            let visible_count = (terminal.size().unwrap().height as usize).saturating_sub(2);
            match event::read().unwrap() {
                Event::Key(key) => {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Up => {
                            if scroll_offset + 1 <= output.len().saturating_sub(visible_count) {
                                scroll_offset += 1;
                            }
                        }
                        KeyCode::Down => {
                            if scroll_offset > 0 {
                                scroll_offset -= 1;
                            }
                        }
                        KeyCode::PageUp => {
                            let increment = visible_count;
                            if scroll_offset + increment <= output.len().saturating_sub(visible_count) {
                                scroll_offset += increment;
                            } else {
                                scroll_offset = output.len().saturating_sub(visible_count);
                            }
                        }
                        KeyCode::PageDown => {
                            if scroll_offset >= visible_count {
                                scroll_offset -= visible_count;
                            } else {
                                scroll_offset = 0;
                            }
                        }
                        _ => {}
                    }
                },
                Event::Mouse(mouse_event) => {
                    match mouse_event.kind {
                        crossterm::event::MouseEventKind::ScrollUp => {
                            if scroll_offset + 1 <= output.len().saturating_sub(visible_count) {
                                scroll_offset += 1;
                            }
                        },
                        crossterm::event::MouseEventKind::ScrollDown => {
                            if scroll_offset > 0 {
                                scroll_offset -= 1;
                            }
                        },
                        _ => {}
                    }
                },
                _ => {}
            }
        }
    }

    // Restore terminal
    disable_raw_mode().unwrap();
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
    terminal.show_cursor().unwrap();
}
