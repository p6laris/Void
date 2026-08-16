use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, FocusTab};
use crate::model::TimerState;
use crate::ui::IconSet;

use super::widgets::{active_task_spans, chip, format_minutes};

/// Streak and daily-goal progress chips shared by the header and zen footer.
fn streak_goal_chips(app: &App, theme: &crate::app::Theme, icons: IconSet) -> Vec<Span<'static>> {
    let today = app.today_focus_mins();
    let goal = app.data.daily_goal_minutes;
    let goal_met = app.daily_goal_met();
    let mut spans = Vec::with_capacity(11);
    spans.extend(chip(
        icons.fire,
        format!("{}d", app.data.streak_days),
        theme.success,
        theme.comment,
    ));
    spans.push(Span::raw(" "));
    spans.extend(chip(
        icons.shield,
        format!(
            "{}/{}",
            app.data.streak_freezes,
            crate::model::STREAK_FREEZE_MAX
        ),
        theme.dim,
        theme.comment,
    ));
    spans.push(Span::raw(" "));
    spans.extend(chip(
        icons.target,
        format!("{}/{}", format_minutes(today), format_minutes(goal)),
        if goal_met { theme.success } else { theme.text },
        theme.comment,
    ));
    spans
}

fn session_total_spans(
    app: &App,
    theme: &crate::app::Theme,
    icons: IconSet,
    as_chip: bool,
) -> Vec<Span<'static>> {
    let count = format!("{}", app.data.total_sessions);
    if as_chip {
        chip(icons.timer, count, theme.dim, theme.comment)
    } else {
        vec![Span::styled(
            format!("{} {count}", icons.timer),
            Style::default().fg(theme.dim),
        )]
    }
}

pub fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;
    let version = env!("CARGO_PKG_VERSION");

    let (timer_icon, timer_color) = match app.timer.state {
        TimerState::Running => (icons.play, theme.accent),
        TimerState::Paused => (icons.pause, theme.warning),
        TimerState::Finished => (icons.check, theme.success),
        TimerState::Idle => (icons.idle, theme.dim),
    };

    let timer_text = if app.timer.state == TimerState::Idle {
        "idle".to_string()
    } else {
        app.timer.format_remaining()
    };

    let mut chips: Vec<Span> = chip(timer_icon, timer_text, timer_color, theme.comment);
    chips.push(Span::raw(" "));
    chips.extend(streak_goal_chips(app, theme, icons));

    if app.timer.state != TimerState::Idle {
        chips.push(Span::raw(" "));
        chips.extend(chip(
            icons.cycle,
            app.timer.cycle_label(),
            theme.info,
            theme.comment,
        ));
    }

    if app.queue_empty() && !app.data.tasks.is_empty() {
        chips.push(Span::raw(" "));
        chips.extend(chip(
            icons.check,
            "queue clear".into(),
            theme.success,
            theme.comment,
        ));
    }

    chips.push(Span::raw(" "));
    chips.extend(session_total_spans(app, theme, icons, false));

    let title = Line::from(vec![
        Span::styled(
            format!(" {} Void ", icons.logo),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("v{version}"), Style::default().fg(theme.dim)),
    ]);

    let right = Line::from(chips);

    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(theme.panel_border))
        .title(title)
        .title_alignment(Alignment::Left)
        .title(right.alignment(Alignment::Right));
    f.render_widget(block, area);
}

pub fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let pending = app.pending_task_count();

    let tabs: Vec<Span> = FocusTab::all()
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let num = i + 1;
            let icon = tab_icon(app.icons, *t);
            let badge = if *t == FocusTab::Tasks && pending > 0 {
                format!(" {pending}")
            } else {
                String::new()
            };
            // Weight and colour carry the active tab; no underline. Terminal underlines
            // render inconsistently and collide with the rule below the tab strip.
            let label = format!("  {icon} {num} {}{badge}  ", t.label().to_lowercase());
            if *t == app.ui.tab {
                Span::styled(
                    label,
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(label, Style::default().fg(theme.dim))
            }
        })
        .collect();

    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(theme.panel_border));
    f.render_widget(Paragraph::new(Line::from(tabs)).block(block), area);
}

fn footer_top_block(theme: &crate::app::Theme) -> Block<'_> {
    Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(theme.panel_border))
}

/// The active-task block on the right of the footer, and the width it wants.
///
/// Split out so the left-hand message can be truncated against what is actually left over
/// rather than being hard-clipped by the layout.
fn footer_right<'a>(app: &'a App, theme: &crate::theme::Theme) -> Option<(Line<'a>, u16)> {
    let spans = active_task_spans(app, theme)?;
    let width = spans.iter().map(|s| s.width() as u16).sum::<u16>() + 1;
    Some((Line::from(spans), width))
}

/// Columns the right-hand block may take. A long task title must not be able to squeeze the
/// status message down to nothing.
fn footer_right_budget(area: Rect, want: u16) -> u16 {
    want.min(area.width / 2)
}

fn draw_footer_bar(f: &mut Frame, app: &App, area: Rect, left: Line<'_>) {
    let theme = &app.theme;
    let mut right_width: u16 = 0;
    let right_line = if let Some((line, want)) = footer_right(app, theme) {
        right_width = footer_right_budget(area, want);
        Some(line)
    } else {
        None
    };

    let footer_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(right_width)])
        .split(area);

    f.render_widget(
        Paragraph::new(left).block(footer_top_block(theme)),
        footer_layout[0],
    );

    if let Some(line) = right_line {
        f.render_widget(
            Paragraph::new(line)
                .block(footer_top_block(theme))
                .alignment(Alignment::Right),
            footer_layout[1],
        );
    }
}

pub fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    let msg = if let Some(ref m) = app.ui.status {
        m.clone()
    } else if app.ui.should_quit {
        format!("{} Goodbye!", app.icons.check)
    } else {
        app.hint()
    };

    let left_style = if app.ui.status_error {
        Style::default().fg(theme.error)
    } else {
        Style::default().fg(theme.dim)
    };

    // Truncated to what the right-hand block leaves behind. A Paragraph in a `Min(1)`
    // column just clips, so the message used to end mid-word with no ellipsis to show it
    // had been cut.
    let right_w = footer_right(app, theme).map_or(0, |(_, w)| footer_right_budget(area, w));
    let budget = area.width.saturating_sub(right_w + 1) as usize;
    let left = Line::from(Span::styled(
        format!(" {}", super::widgets::truncate(&msg, budget)),
        left_style,
    ));
    draw_footer_bar(f, app, area, left);
}

fn tab_icon(icons: IconSet, tab: FocusTab) -> &'static str {
    match tab {
        FocusTab::Dashboard => icons.dashboard,
        FocusTab::Tasks => icons.tasks,
        FocusTab::Stats => icons.stats,
        FocusTab::Settings => icons.settings,
        FocusTab::Help => icons.help,
        FocusTab::About => icons.about,
    }
}

pub fn draw_zen_footer(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;

    let mut chips = streak_goal_chips(app, theme, icons);
    chips.push(Span::raw(" "));
    chips.extend(session_total_spans(app, theme, icons, true));

    draw_footer_bar(f, app, area, Line::from(chips));
}
