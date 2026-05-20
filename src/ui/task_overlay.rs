use crate::app::App;
use crate::locale::t;
use crate::model::{AgentSession, SessionStatus};
use crate::theme::Theme;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::truncate_str;

const MIN_WIDTH: u16 = 40;
const MAX_HEIGHT: u16 = 12;

pub(crate) fn draw_task_overlay(f: &mut Frame, app: &App, theme: &Theme) {
    let area = f.area();
    if area.width < MIN_WIDTH || area.height < 2 {
        return;
    }

    let popup_w = MIN_WIDTH.max(area.width.saturating_mul(60) / 100);
    let desired_h = app.sessions.len() as u16 + 2;
    let popup_h = desired_h.min(MAX_HEIGHT).min(area.height);
    let popup = Rect::new(area.width.saturating_sub(popup_w), 0, popup_w, popup_h);

    f.render_widget(Clear, popup);

    let block = Block::default()
        .style(Style::default().bg(theme.main_bg))
        .title(
            Line::from(vec![Span::styled(
                t("task_overlay.title"),
                Style::default()
                    .fg(theme.title)
                    .add_modifier(Modifier::BOLD),
            )])
            .alignment(Alignment::Center),
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.cpu_box));
    f.render_widget(block, popup);

    let inner = Rect::new(
        popup.x + 1,
        popup.y + 1,
        popup.width.saturating_sub(2),
        popup.height.saturating_sub(2),
    );
    if inner.height == 0 {
        return;
    }

    let visible_rows = inner.height as usize;
    let selected = app
        .task_overlay_selected
        .min(app.sessions.len().saturating_sub(1));
    let start = selected.saturating_add(1).saturating_sub(visible_rows);
    let end = (start + visible_rows).min(app.sessions.len());

    let mut lines = Vec::new();
    for (index, session) in app.sessions[start..end].iter().enumerate() {
        let session_index = start + index;
        let selected = session_index == app.task_overlay_selected;
        lines.push(session_line(
            session,
            app,
            inner.width as usize,
            selected,
            theme,
        ));
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            t("task_overlay.no_sessions"),
            Style::default().fg(theme.inactive_fg),
        )));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn session_line(
    session: &AgentSession,
    app: &App,
    width: usize,
    selected: bool,
    theme: &Theme,
) -> Line<'static> {
    let name = app
        .summaries
        .get(&session.session_id)
        .map(String::as_str)
        .filter(|summary| !summary.is_empty())
        .unwrap_or(&session.project_name);
    let task = session
        .current_tasks
        .last()
        .map(String::as_str)
        .filter(|task| !task.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| t("misc.dash"));

    let prefix_w = 2;
    let name_w = 16usize.min(width.saturating_sub(prefix_w + 2));
    let task_w = width.saturating_sub(prefix_w + name_w + 2);
    let base_style = if selected {
        Style::default()
            .fg(theme.selected_fg)
            .bg(theme.selected_bg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.main_fg)
    };
    let status_style = base_style.fg(status_color(&session.status));

    Line::from(vec![
        Span::styled("● ", status_style),
        Span::styled(
            format!("{:<width$} ", truncate_str(name, name_w), width = name_w),
            base_style,
        ),
        Span::styled(truncate_str(&task, task_w), base_style),
    ])
}

fn status_color(status: &SessionStatus) -> Color {
    match status {
        SessionStatus::Thinking => Color::Yellow,
        SessionStatus::Executing => Color::Green,
        SessionStatus::Waiting => Color::Blue,
        SessionStatus::RateLimited => Color::Red,
        SessionStatus::Done => Color::DarkGray,
    }
}
