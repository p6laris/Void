mod calendar;
mod chrome;
pub mod heatmap;
mod icons;
mod stats;
mod widgets;

pub mod about;
pub mod dashboard;
pub mod help;
pub mod popups;
pub mod settings;
pub mod tasks;
pub mod zen;

use about::*;
use dashboard::*;
use help::*;
use popups::*;
use settings::*;
use tasks::*;
use zen::*;

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, Wrap,
};
use ratatui::Frame;

use crate::app::{App, FocusTab, InputMode, TaskFilter};
use crate::canvas_timer::{
    draw_break_tip, draw_dashboard_canvas, draw_zen_canvas, format_time_stack, session_dots,
    DashboardSceneOptions, ZenSceneOptions,
};
use crate::model::TimerMode;
use crate::theme::Theme;

pub use icons::{IconMode, IconSet};

use chrome::{draw_footer, draw_header, draw_tabs};
use stats::draw_stats;
use widgets::{
    active_task_spans, centered_rect, chip, comment_line, dense_panel, format_minutes, meta_row,
    section_title, status_checkbox, tag_span, task_status_color, text_gauge, truncate,
    vertical_rule,
};

pub fn render(f: &mut Frame, app: &mut App) {
    app.refresh_frame_today_cache();
    let area = f.area();
    for cell in f.buffer_mut().content.iter_mut() {
        cell.set_bg(app.theme.bg);
    }

    if app.ui.zen_mode && app.ui.tab == FocusTab::Dashboard {
        draw_zen_dashboard(f, app, area);
        draw_popup(f, app);
        return;
    }

    let footer_h = 2;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Min(6),
            Constraint::Length(footer_h),
        ])
        .split(area);

    draw_header(f, app, chunks[0]);
    draw_tabs(f, app, chunks[1]);

    match app.ui.tab {
        FocusTab::Dashboard => draw_dashboard(f, app, chunks[2]),
        FocusTab::Tasks => draw_tasks(f, app, chunks[2]),
        FocusTab::Stats => draw_stats(f, app, chunks[2]),
        FocusTab::Settings => draw_settings(f, app, chunks[2]),
        FocusTab::Help => draw_help(f, app, chunks[2]),
        FocusTab::About => draw_about(f, app, chunks[2]),
    }

    draw_footer(f, app, chunks[3]);

    draw_popup(f, app);
    if matches!(app.input.input_mode, InputMode::Editing)
        && !matches!(
            app.input.popup.as_ref(),
            Some(crate::app::Popup::AddTask)
                | Some(crate::app::Popup::EditTask(_))
                | Some(crate::app::Popup::AddSubtask(_))
                | Some(crate::app::Popup::EditSubtask(_, _))
        )
    {
        draw_input(f, app, chunks[2]);
    }
}
