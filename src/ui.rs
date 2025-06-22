use crate::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, Paragraph, Wrap,
    },
    Frame,
};

pub fn ui(f: &mut Frame, app: &mut App) {
    if app.fullscreen_list.is_some() {
        render_fullscreen_list(f, app);
        return;
    }
    if let Some(output) = &app.fullscreen_output {
        let text = format!("{}\n\n[ Press any key to return ]", output);
        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(app.config.get_color_scheme().get_color("primary")))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, f.size());
        return;
    }

    app.update_status_timer();

    let status_bar_height = if let Some(msg) = &app.status_message {
        msg.lines().count() as u16 + 1
    } else if let Some(_) = &app.confirmation_message {
        4 // Extra space for confirmation message
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

    let list = create_list(app);
    f.render_widget(list, chunks[1]);

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

fn create_list(app: &App) -> List {
    let color_scheme = app.config.get_color_scheme();
    let list_items: Vec<ListItem> = app.get_current_list_items()
        .into_iter()
        .enumerate()
        .map(|(i, name)| {
            let is_selected = match app.view_level {
                0 => i == app.selected_indices[0],
                1 => i == app.selected_indices[1],
                _ => false,
            };

            let mut spans = vec![];
            if is_selected {
                spans.push(Span::styled("> ", Style::default().fg(color_scheme.get_color("primary"))));
            } else {
                spans.push(Span::from("  "));
            }

            let style = if is_selected {
                Style::default().fg(color_scheme.get_color("primary")).add_modifier(Modifier::BOLD)
            } else if app.view_level == 1 && !name.starts_with("  ") { // Sub-category
                Style::default().fg(color_scheme.get_color("secondary")).add_modifier(Modifier::BOLD)
            } else { // Top-level category or tweak option
                Style::default().fg(color_scheme.get_color("text_dim"))
            };
            let owned_name = name.trim().to_string();
            spans.push(Span::styled(owned_name, style));
            
            if app.applied_tweaks.contains(&name) {
                spans.push(Span::styled(" ✓", Style::default().fg(color_scheme.get_color("success"))));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    List::new(list_items)
}

fn create_status_bar(app: &App) -> Paragraph {
    let color_scheme = app.config.get_color_scheme();
    let status_text = if let Some(message) = &app.status_message {
        message.clone()
    } else if let Some(confirmation) = &app.confirmation_message {
        format!("{}\nInput: {}", confirmation, app.input_buffer)
    } else {
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
        }
    };

    let style = if app.confirmation_message.is_some() {
        Style::default().fg(color_scheme.get_color("error")).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(color_scheme.get_color("primary"))
    };

    Paragraph::new(status_text)
        .style(style)
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: true })
} 