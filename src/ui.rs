use crate::app::{App, Tile};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, Paragraph, Wrap,
    },
    Frame,
};

pub fn ui(f: &mut Frame, app: &mut App) {
    if app.sokoban_game.is_some() {
        render_sokoban_game(f, app);
        return;
    }
    if app.fullscreen_list.is_some() {
        render_fullscreen_list(f, app);
        return;
    }
    if let Some(output) = &app.fullscreen_output {
        let text = format!("{}\n\n[ Press any key to return, ↑/↓ to scroll ]", output);
        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(app.config.get_color_scheme().get_color("primary")))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true })
            .scroll((app.fullscreen_output_scroll, 0));
        f.render_widget(paragraph, f.size());
        return;
    }

    app.update_status_timer();

    let status_bar_height = if app.text_input_prompt.is_some() {
        4
    } else if app.confirmation_message.is_some() {
        4
    } else if let Some(msg) = &app.status_message {
        msg.lines().count() as u16 + 1
    } else {
        2
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(2),
                Constraint::Min(0),
                Constraint::Length(status_bar_height),
            ]
            .as_ref(),
        )
        .split(f.size());

    let header = create_header(app);
    f.render_widget(header, chunks[0]);

    render_main_list(f, app, chunks[1]);

    let status = create_status_bar(app);
    f.render_widget(status, chunks[2]);
}

fn render_fullscreen_list(f: &mut Frame, app: &mut App) {
    let list_items_str = app.fullscreen_list.as_ref().unwrap();
    let items: Vec<ListItem> = list_items_str
        .iter()
        .map(|item| {
            ListItem::new(vec![Line::from(Span::raw(item))])
        })
        .collect();

    let list_widget = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(app.fullscreen_list_title.clone()))
        .highlight_style(Style::default().fg(app.config.get_color_scheme().get_color("primary")).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");

    f.render_stateful_widget(list_widget, f.size(), &mut app.fullscreen_list_state);
}

fn create_header(app: &App) -> Paragraph {
    let header_text = format!("macOS-tweaks");
    
    Paragraph::new(header_text)
        .style(Style::default().fg(app.config.get_color_scheme().get_color("primary")).add_modifier(Modifier::BOLD))
        .alignment(ratatui::layout::Alignment::Center)
}

fn render_main_list(f: &mut Frame, app: &mut App, area: Rect) {
    let color_scheme = app.config.get_color_scheme();
    let list_items: Vec<ListItem> = app.get_current_list_items()
        .into_iter()
        .map(|name| {
            let style = if app.view_level == 1 && !name.starts_with("  ") { // Sub-category
                Style::default().fg(color_scheme.get_color("secondary")).add_modifier(Modifier::BOLD)
            } else { // Top-level category or tweak option
                Style::default().fg(color_scheme.get_color("text_dim"))
            };
            let owned_name = name.trim().to_string();
            
            let mut spans = vec![Span::styled(owned_name, style)];
            
            if app.applied_tweaks.contains(&name) {
                spans.push(Span::styled(" ✗", Style::default().fg(color_scheme.get_color("success"))));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(list_items)
        .highlight_style(Style::default().fg(color_scheme.get_color("primary")).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");

    let state = if app.view_level == 0 {
        &mut app.category_list_state
    } else {
        &mut app.tweak_list_state
    };

    f.render_stateful_widget(list, area, state);
}

fn create_status_bar(app: &App) -> Paragraph {
    let color_scheme = app.config.get_color_scheme();
    let (status_text, style) = if let Some(prompt) = &app.text_input_prompt {
        (
            format!("{} (Enter to confirm, Esc to cancel)\nInput: {}", prompt, app.input_buffer),
            Style::default().fg(color_scheme.get_color("primary")).add_modifier(Modifier::BOLD),
        )
    } else if let Some(confirmation) = &app.confirmation_message {
        (
            format!("{}\nInput: {}", confirmation, app.input_buffer),
            Style::default().fg(color_scheme.get_color("error")).add_modifier(Modifier::BOLD),
        )
    } else if let Some(message) = &app.status_message {
        (message.clone(), Style::default().fg(color_scheme.get_color("primary")))
    } else {
        (
            match app.view_level {
                0 => "Navigation: ↑↓ to select, → or Enter to view category, q to quit".to_string(),
                1 => {
                    if app.viewing_sub_category.is_some() {
                        "Navigation: ↑↓ to select, Enter to apply, ← to go back, q to quit".to_string()
                    } else {
                        "Navigation: ↑↓ to select, → or Enter to view options, ← to go back, q to quit".to_string()
                    }
                },
                _ => "".to_string(),
            },
            Style::default().fg(color_scheme.get_color("primary")),
        )
    };

    Paragraph::new(status_text)
        .style(style)
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: true })
}

fn render_sokoban_game(f: &mut Frame, app: &mut App) {
    let game = app.sokoban_game.as_mut().unwrap();
    let color_scheme = app.config.get_color_scheme();

    let title = if game.is_complete {
        format!("Sokoban - Level Complete! ({} moves) - R to restart, Q to quit", game.moves)
    } else {
        format!("Sokoban - Moves: {} - WASD/Arrows to move, R to restart, Q to quit", game.moves)
    };

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(color_scheme.get_color("primary")));
    
    let game_area = f.size();
    f.render_widget(outer_block, game_area);

    // Center the game board
    let game_width = game.level[0].len() as u16 * 2;
    let game_height = game.level.len() as u16;
    let centered_rect = Rect {
        x: game_area.x + (game_area.width.saturating_sub(game_width)) / 2,
        y: game_area.y + (game_area.height.saturating_sub(game_height)) / 2,
        width: game_width,
        height: game_height,
    };
    
    for (y, row) in game.level.iter().enumerate() {
        for (x, tile) in row.iter().enumerate() {
            let (mut char, mut style) = match tile {
                Tile::Wall => ("🧱", Style::default()),
                Tile::Floor => ("  ", Style::default()),
                Tile::Target => ("🎯", Style::default()),
            };

            if game.player == (x, y) {
                char = "🧑";
                style = style.add_modifier(Modifier::BOLD);
            } else if game.boxes.contains(&(x, y)) {
                if let Tile::Target = game.level[y][x] {
                    char = "✅"; // Box on a target
                    style = style.add_modifier(Modifier::BOLD);
                } else {
                    char = "📦"; // Box on floor
                }
            }
            
            let rect = Rect::new(centered_rect.x + (x * 2) as u16, centered_rect.y + y as u16, 2, 1);
            f.render_widget(Paragraph::new(char).style(style), rect);
        }
    }
} 