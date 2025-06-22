mod app;
mod ui;
mod tweaks;
mod utils;
mod config;

use anyhow::Result;
use app::App;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::io;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Lists all available, runnable tweaks
    List,
    /// Applies a specific tweak by name
    Apply {
        /// The name of the tweak to apply
        name: String,
    },
    /// Reverts a specific tweak by name
    Revert {
        /// The name of the tweak to revert
        name: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(command) = cli.command {
        let app = App::new();
        match command {
            Commands::List => {
                println!("Available tweaks:");
                for category in &app.categories {
                    let runnable_tweaks: Vec<_> = category
                        .tweaks
                        .iter()
                        .filter(|t| !t.enable_command.is_empty() && !t.enable_command.starts_with("__"))
                        .collect();

                    if !runnable_tweaks.is_empty() {
                        println!("\n{}:", category.name);
                        for tweak in runnable_tweaks {
                            println!("  - {}", tweak.name.trim());
                        }
                    }
                }
            }
            Commands::Apply { name } => {
                if let Some(tweak) = app.find_tweak_by_name(&name) {
                    if tweak.enable_command.is_empty() || tweak.enable_command.starts_with("__") {
                        println!("Tweak '{}' is a category or not directly runnable.", name);
                    } else {
                        println!("Applying tweak: '{}'", name);
                        utils::execute_command(&tweak.enable_command, true)?;
                        println!("Successfully applied tweak: '{}'", name);
                    }
                } else {
                    eprintln!("Tweak not found: '{}'", name);
                }
            }
            Commands::Revert { name } => {
                if let Some(tweak) = app.find_tweak_by_name(&name) {
                    if tweak.disable_command.is_empty() {
                        eprintln!("Revert command not available for tweak: '{}'", name);
                    } else {
                        println!("Reverting tweak: '{}'", name);
                        utils::execute_command(&tweak.disable_command, true)?;
                        println!("Successfully reverted tweak: '{}'", name);
                    }
                } else {
                    eprintln!("Tweak not found: '{}'", name);
                }
            }
        }
        return Ok(());
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend + std::io::Write>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui::ui(f, app))?;

        if app.should_quit {
            break;
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if app.fullscreen_list.is_some() {
                        handle_fullscreen_list_nav(app, key.code, terminal, |t, cmd| run_interactive_command(t, cmd))?;
                        continue;
                    }
                    if app.fullscreen_output.is_some() {
                        app.fullscreen_output = None;
                        continue;
                    }
                    if app.confirmation_message.is_some() {
                        match key.code {
                            KeyCode::Char(c) => app.input_buffer.push(c),
                            KeyCode::Backspace => { app.input_buffer.pop(); },
                            KeyCode::Enter => {
                                let input = app.input_buffer.clone();
                                app.handle_confirmation(&input, terminal, |t, cmd| run_interactive_command(t, cmd))?;
                                app.input_buffer.clear();
                            },
                            KeyCode::Esc => {
                                app.handle_confirmation("no", terminal, |t, cmd| run_interactive_command(t, cmd))?;
                                app.input_buffer.clear();
                            },
                            _ => {}
                        }
                    } else {
                        handle_main_tab(app, key.code, terminal)?;
                    }
                },
                Event::Mouse(_) => {}, // Ignore mouse events
                _ => {} // Ignore other events
            }
        }
    }

    Ok(())
}

fn handle_main_tab<B: Backend + std::io::Write>(app: &mut App, key_code: KeyCode, terminal: &mut Terminal<B>) -> Result<()> {
    match key_code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Enter => app.apply_selected_tweak(terminal, |t, cmd| run_interactive_command(t, cmd))?,
        KeyCode::Right => app.handle_right_key(),
        KeyCode::Left => app.handle_left_key(),
        KeyCode::Up => app.previous_item(),
        KeyCode::Down => app.next_item(),
        _ => {}
    }
    Ok(())
}

fn run_interactive_command<B: Backend + std::io::Write>(terminal: &mut Terminal<B>, command: &str) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    let result = utils::execute_command(command, true);

    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    terminal.hide_cursor()?;
    terminal.clear()?;

    result.map(|_| ())
}

fn handle_fullscreen_list_nav<B: Backend + std::io::Write>(
    app: &mut App,
    key_code: KeyCode,
    terminal: &mut Terminal<B>,
    run_interactive: impl Fn(&mut Terminal<B>, &str) -> Result<()>,
) -> Result<()> {
    match key_code {
        KeyCode::Up => {
            if let Some(selected) = app.fullscreen_list_state.selected() {
                if selected > 0 {
                    app.fullscreen_list_state.select(Some(selected - 1));
                }
            }
        },
        KeyCode::Down => {
            if let (Some(selected), Some(list)) = (app.fullscreen_list_state.selected(), &app.fullscreen_list) {
                if selected < list.len() - 1 {
                    app.fullscreen_list_state.select(Some(selected + 1));
                }
            }
        },
        KeyCode::Enter => {
            if let (Some(list), Some(selected_index)) = (app.fullscreen_list.clone(), app.fullscreen_list_state.selected()) {
                let selected_item = &list[selected_index];
                let command = if app.fullscreen_list_title.contains("Outdated") {
                    format!("brew upgrade {}", selected_item)
                } else {
                    format!("brew info {}", selected_item)
                };
                app.fullscreen_list = None;
                run_interactive(terminal, &command)?;
            }
        },
        KeyCode::Esc | KeyCode::Char('q') => {
            app.fullscreen_list = None;
        },
        _ => {}
    }
    Ok(())
}
