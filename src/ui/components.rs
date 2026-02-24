use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use crate::ui::App;

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(20),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(frame.size());

    render_chat_history(frame, chunks[0], app);
    render_tool_output(frame, chunks[1], app);
    render_input(frame, chunks[2], app);
    render_status_bar(frame, chunks[3], app);
}

fn render_chat_history(frame: &mut Frame, area: Rect, app: &App) {
    let messages = app.shared_messages.take_messages();
    let message_count = messages.len();

    let lines: Vec<Line> = messages
        .iter()
        .map(|msg| Line::from(msg.clone()))
        .collect();

    let history = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Chat"))
        .wrap(Wrap { trim: true })
        .scroll((message_count as u16, 0));

    frame.render_widget(history, area);
}

fn render_tool_output(frame: &mut Frame, area: Rect, app: &App) {
    let output = Paragraph::new(app.tool_output.as_str())
        .block(Block::default().borders(Borders::ALL).title("Tool Output"))
        .wrap(Wrap { trim: true });

    frame.render_widget(output, area);
}

fn render_input(frame: &mut Frame, area: Rect, app: &App) {
    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title("Input"))
        .style(Style::default());

    frame.render_widget(input, area);
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let status_text = format!(
        "nano code | Model: {} | Provider: {} | Plugins: {} | Press Ctrl+C to quit",
        app.status.model, app.status.provider, app.status.plugins_loaded
    );

    let status = Paragraph::new(status_text)
        .block(Block::default())
        .style(Style::default().fg(Color::Yellow));

    frame.render_widget(status, area);
}