#![allow(clippy::too_many_arguments)]
use std::f64::consts::PI;

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Color;
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::canvas::{Canvas, Circle, Points};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;

use crate::model::{TimerMode, TimerState};
use crate::timer::Timer;

const ARC_STEPS: usize = 360;
const PARTICLE_COUNT: usize = 12;

// ── public types ─────────────────────────────────────────────────────────────

pub struct TimerCanvasStyle {
    pub track: Color,
    pub progress: Color,
    pub progress_dim: Color,
    pub task_track: Color,
    pub task_progress: Color,
    pub cap: Color,
    pub text: Color,
    pub dim: Color,
}

pub struct TimerCanvasOptions {
    pub task_progress: Option<f64>,
    pub breathe: bool,
}

impl Default for TimerCanvasOptions {
    fn default() -> Self {
        Self {
            task_progress: None,
            breathe: true,
        }
    }
}

/// Palette for the dashboard / zen timer canvas scene.
pub struct SceneStyle {
    pub mode: Color,
    pub track: Color,
    pub task: Color,
    pub task_dim: Color,
    pub bg: Color,
    pub bg_mid: Color,
    pub bg_light: Color,
    pub wave: Color,
    pub core: Color,
    pub glow: Color,
    pub particle: Color,
    pub text: Color,
    pub session_on: Color,
    pub session_off: Color,
}

pub type DashboardSceneStyle = SceneStyle;
pub type ZenSceneStyle = SceneStyle;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SceneLayout {
    Dashboard,
    Zen,
}

#[derive(Clone, Copy)]
pub struct SceneOptions {
    pub task_progress: Option<f64>,
    pub pending_tasks: u32,
    pub active_task_index: Option<u32>,
    pub sessions_done: u32,
    pub sessions_total: u32,
    pub layout: SceneLayout,
}

pub type DashboardSceneOptions = SceneOptions;
pub type ZenSceneOptions = SceneOptions;

// ── break wellness tips ──────────────────────────────────────────────────────

const BREAK_TIPS: &[&str] = &[
    "Look at something 20 feet (6 m) away for 20 seconds — the 20-20-20 rule protects your eyes.",
    "Stand up and roll your shoulders back slowly for 30 seconds.",
    "Blink slowly 10 times to re-wet tired eyes.",
    "Take 4 deep breaths: inhale 4 s, hold 4 s, exhale 6 s.",
    "Walk to a window and focus on the farthest object you can see.",
    "Gently turn your neck left and right — never force the stretch.",
    "Close your eyes for 20 seconds and let them fully rest.",
    "Stand and do 10 calf raises to boost leg circulation.",
    "Roll your wrists clockwise, then counterclockwise.",
    "Massage your temples with slow, gentle circular motions.",
    "Drink a glass of water — hydration helps focus and energy.",
    "Reach your arms overhead and stretch your whole spine.",
    "Look outside at greenery — natural scenes relax eye muscles.",
    "Unclench your jaw and let your tongue rest on the roof of your mouth.",
    "Stand, touch your toes, or do a gentle forward fold for 20 seconds.",
    "Focus on a distant horizon line to relax your ciliary muscles.",
    "Open a window for fresh air and take three slow breaths.",
    "Stretch your fingers wide, then make a fist — repeat 8 times.",
    "Shift your gaze between near and far objects three times.",
    "Stand up every break — sitting too long strains your back and hips.",
];

const TIP_SLOT_SECS: f64 = 9.0;

#[derive(Debug, Clone)]
pub struct BreakTip {
    pub text: String,
    pub fade: f64,
    pub reveal: f64,
}

pub fn current_break_tip(timer: &Timer) -> Option<BreakTip> {
    match timer.mode {
        TimerMode::ShortBreak | TimerMode::LongBreak => {}
        _ => return None,
    }

    if matches!(timer.state, TimerState::Idle) {
        return Some(BreakTip {
            text: "Press start — use this break to rest your eyes and body.".into(),
            fade: 0.65,
            reveal: 1.0,
        });
    }

    let elapsed = timer.current_elapsed_secs_f64();
    let slot = TIP_SLOT_SECS;
    let idx = (elapsed / slot).floor() as usize % BREAK_TIPS.len();
    let phase = (elapsed % slot) / slot;

    Some(BreakTip {
        text: BREAK_TIPS[idx].to_string(),
        fade: tip_fade(phase),
        reveal: tip_reveal(phase),
    })
}

fn tip_fade(phase: f64) -> f64 {
    const IN: f64 = 0.12;
    const OUT: f64 = 0.88;
    if phase < IN {
        smoothstep(phase / IN)
    } else if phase > OUT {
        smoothstep((1.0 - phase) / (1.0 - OUT))
    } else {
        1.0
    }
}

fn tip_reveal(phase: f64) -> f64 {
    const IN: f64 = 0.35;
    if phase < IN {
        smoothstep(phase / IN)
    } else {
        1.0
    }
}

pub fn draw_break_tip(
    f: &mut Frame,
    area: Rect,
    timer: &Timer,
    accent: Color,
    text: Color,
    dim: Color,
    heart: &str,
) {
    if area.height == 0 || area.width < 4 {
        return;
    }
    let Some(tip) = current_break_tip(timer) else {
        return;
    };

    let visible_chars = ((tip.text.chars().count() as f64) * tip.reveal).ceil() as usize;
    let shown: String = tip.text.chars().take(visible_chars).collect();
    let fg = blend_color(dim, text, tip.fade);
    let prefix_fg = blend_color(dim, accent, tip.fade);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{} ", heart),
                ratatui::style::Style::default().fg(prefix_fg),
            ),
            Span::styled(shown, ratatui::style::Style::default().fg(fg)),
        ]))
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Center),
        area,
    );
}

// ── timer scene ──────────────────────────────────────────────────────────────

pub fn draw_dashboard_canvas(
    f: &mut Frame,
    area: Rect,
    timer: &Timer,
    style: &DashboardSceneStyle,
    options: &DashboardSceneOptions,
) {
    let mut opts = *options;
    opts.layout = SceneLayout::Dashboard;
    draw_scene_canvas(f, area, timer, style, &opts);
}

pub fn draw_zen_canvas(
    f: &mut Frame,
    area: Rect,
    timer: &Timer,
    style: &ZenSceneStyle,
    options: &ZenSceneOptions,
) {
    let mut opts = *options;
    opts.layout = SceneLayout::Zen;
    draw_scene_canvas(f, area, timer, style, &opts);
}

pub fn draw_scene_canvas(
    f: &mut Frame,
    area: Rect,
    timer: &Timer,
    style: &SceneStyle,
    options: &SceneOptions,
) {
    if area.width < 8 || area.height < 4 {
        return;
    }

    let remaining = (1.0 - timer.progress()).clamp(0.0, 1.0);
    let motion = scene_motion(timer);
    let (xb, yb) = square_bounds(area);
    let marker = canvas_marker(area);
    let t = time_s();
    let zen = options.layout == SceneLayout::Zen;
    let compact = !zen;
    let extent = canvas_extent(xb, yb);
    let base_r = fit_base_r(extent, options.layout);

    let bg = style.bg;
    let bg_mid = style.bg_mid;
    let bg_light = style.bg_light;
    let wave = style.wave;
    let core = style.core;
    let glow = style.glow;
    let particle = style.particle;
    let mode = style.mode;
    let track = style.track;
    let task = style.task;
    let task_dim = style.task_dim;
    let text = style.text;
    let session_on = style.session_on;
    let session_off = style.session_off;
    let pending = options.pending_tasks;
    let active_idx = options.active_task_index;
    let task_prog = options.task_progress;
    let sessions_done = options.sessions_done;
    let sessions_total = options.sessions_total;
    let timer_state = timer.state;
    let timer_mode = timer.mode;

    let canvas = Canvas::default()
        .marker(marker)
        .x_bounds(xb)
        .y_bounds(yb)
        .paint(move |ctx| {
            let (cx, cy) = center(xb, yb);
            let breath = motion.breath;
            let base = base_r * motion.scale * (0.86 + 0.14 * remaining);

            if compact {
                draw_bg_wash_compact(ctx, cx, cy, base, bg, bg_mid);
            } else {
                draw_bg_wash(ctx, (xb, yb), (cx, cy, base), (bg, bg_mid, bg_light));
            }
            draw_bg_waves(
                ctx,
                (xb, yb),
                (cx, cy, base, t),
                motion,
                (wave, bg_mid),
                timer_mode,
                compact,
            );
            draw_particles(
                ctx,
                (cx, cy, base, t),
                motion,
                (particle, bg),
                if compact {
                    5
                } else if zen && pending == 0 {
                    PARTICLE_COUNT + 4
                } else {
                    PARTICLE_COUNT
                },
            );

            if pending == 0 && !compact {
                let moon_x = if zen {
                    cx + base * 0.95
                } else {
                    cx - base * 0.88
                };
                draw_idle_marker(ctx, moon_x, cy - base * 0.72, t, glow, core);
            } else if zen {
                draw_task_constellation(
                    ctx,
                    (cx, cy, base, t),
                    motion,
                    pending,
                    active_idx,
                    (task, task_dim, bg),
                );
            } else {
                draw_task_orbit(
                    ctx,
                    (cx, cy, base, t),
                    motion,
                    pending,
                    active_idx,
                    (task, task_dim, bg),
                );
            }

            draw_soft_progress_wreath(
                ctx,
                (cx, cy, base * if zen { 1.18 } else { 1.12 }, t),
                remaining,
                (track, mode),
                timer_state,
                compact,
            );

            draw_timer_orb(
                ctx,
                (cx, cy, base, breath),
                motion,
                (bg, glow, core, mode),
                timer_state,
                timer_mode,
                compact,
            );

            if let Some(tp) = task_prog {
                if tp > 0.001 {
                    draw_task_fill(ctx, cx, cy, base * 0.38, tp, task, bg);
                }
            }

            if sessions_total > 0 {
                draw_session_stars(
                    ctx,
                    (cx, cy, base, t),
                    sessions_done,
                    sessions_total,
                    (session_on, session_off, bg),
                    if zen {
                        SessionArc::Top
                    } else {
                        SessionArc::Bottom
                    },
                );
            }

            if timer_state == TimerState::Paused {
                draw_soft_pause(ctx, cx, cy, base * 0.22, text);
            }

            if timer_state == TimerState::Finished {
                draw_completion_shimmer(ctx, cx, cy, base, t, mode, glow);
            }
        });

    f.render_widget(canvas, area);
}

#[derive(Clone, Copy)]
struct SceneMotion {
    breath: f64,
    speed: f64,
    scale: f64,
    glow: f64,
}

fn scene_motion(timer: &Timer) -> SceneMotion {
    let t = time_s();
    let on_break = timer.mode.is_break();
    let breath_hz = if on_break { 0.18 } else { 0.22 };
    let breath = 0.5 + 0.5 * (t * breath_hz * 2.0 * PI).sin();

    match timer.state {
        TimerState::Running => SceneMotion {
            breath,
            speed: if on_break { 0.55 } else { 1.0 },
            scale: 1.0 + 0.018 * (t * 0.9).sin(),
            glow: 1.0,
        },
        TimerState::Idle => SceneMotion {
            breath,
            speed: 0.45,
            scale: 0.98 + 0.025 * (t * 0.55).sin(),
            glow: 0.88,
        },
        TimerState::Paused => SceneMotion {
            breath: 0.52,
            speed: 0.05,
            scale: 0.94,
            glow: 0.42,
        },
        TimerState::Finished => SceneMotion {
            breath: 0.5 + 0.5 * (t * 1.4).sin(),
            speed: 0.7,
            scale: 1.04 + 0.025 * (t * 1.8).sin(),
            glow: 1.15,
        },
    }
}

fn canvas_extent(xb: [f64; 2], yb: [f64; 2]) -> f64 {
    (xb[1] - xb[0]).min(yb[1] - yb[0])
}

fn fit_base_r(extent: f64, layout: SceneLayout) -> f64 {
    let frac = match layout {
        SceneLayout::Zen => 0.36,
        SceneLayout::Dashboard => 0.30,
    };
    (extent * frac).clamp(7.0, 34.0)
}

fn draw_bg_wash(
    ctx: &mut ratatui::widgets::canvas::Context,
    bounds: ([f64; 2], [f64; 2]),
    geom: (f64, f64, f64),
    colors: (Color, Color, Color),
) {
    let (xb, yb) = bounds;
    let (cx, cy, base) = geom;
    let (bg, bg_mid, bg_light) = colors;
    let w = xb[1] - xb[0];
    let h = yb[1] - yb[0];
    let bands = [(0.18, bg), (0.42, bg_mid), (0.68, bg_light)];
    for (frac, color) in bands {
        let ry = cy - base * 0.2 + h * frac * 0.35;
        let rx = w * 0.52;
        draw_soft_ellipse(ctx, cx, ry, rx, base * 0.55, color, bg);
    }
}

fn draw_bg_wash_compact(
    ctx: &mut ratatui::widgets::canvas::Context,
    cx: f64,
    cy: f64,
    base: f64,
    bg: Color,
    bg_mid: Color,
) {
    draw_soft_ellipse(
        ctx,
        cx,
        cy - base * 0.15,
        base * 1.35,
        base * 1.05,
        bg_mid,
        bg,
    );
}

fn draw_soft_ellipse(
    ctx: &mut ratatui::widgets::canvas::Context,
    cx: f64,
    cy: f64,
    rx: f64,
    ry: f64,
    color: Color,
    bg: Color,
) {
    let steps = 72;
    let mut coords = [(0.0, 0.0); 72];
    for (i, coord) in coords.iter_mut().enumerate().take(steps) {
        let a = 2.0 * PI * i as f64 / steps as f64;
        *coord = (cx + a.cos() * rx, cy + a.sin() * ry);
    }
    ctx.draw(&Points {
        coords: &coords,
        color: blend_color(bg, color, 0.55),
    });
}

fn draw_bg_waves(
    ctx: &mut ratatui::widgets::canvas::Context,
    bounds: ([f64; 2], [f64; 2]),
    geom: (f64, f64, f64, f64),
    motion: SceneMotion,
    colors: (Color, Color),
    mode: TimerMode,
    compact: bool,
) {
    let (xb, _yb) = bounds;
    let (_cx, cy, base, t) = geom;
    let (wave, bg) = colors;
    let w = xb[1] - xb[0];
    let on_break = mode.is_break();
    let bands = if compact {
        1
    } else if on_break {
        2
    } else {
        3
    };
    let samples = if compact { 40 } else { 64 };

    for band in 0..bands {
        let mut coords = [(0.0, 0.0); 64];
        let y0 = cy - base * 0.55 + band as f64 * 5.5;
        let phase = t * motion.speed * 0.12 + band as f64 * 1.7;
        for (i, coord) in coords.iter_mut().enumerate().take(samples) {
            let x = xb[0] + w * i as f64 / (samples - 1) as f64;
            let wave = (x * 0.06 + phase).sin() * 3.5 + (x * 0.025 + phase * 1.4).sin() * 2.0;
            *coord = (x, y0 + wave);
        }
        let mix = 0.28 + band as f64 * 0.12;
        ctx.draw(&Points {
            coords: &coords[..samples],
            color: blend_color(bg, wave, mix * motion.glow),
        });
    }
}

fn draw_particles(
    ctx: &mut ratatui::widgets::canvas::Context,
    geom: (f64, f64, f64, f64),
    motion: SceneMotion,
    colors: (Color, Color),
    count: usize,
) {
    let (cx, cy, base, t) = geom;
    let (particle, bg) = colors;
    let count = count.min(PARTICLE_COUNT + 4);
    let spread = base * 1.45;

    for i in 0..count {
        let seed = i as f64 * 2.399_963_229_728_653;
        let fx = cx + (seed * 1.7 + t * (0.08 + i as f64 * 0.008)).sin() * spread;
        let fy = cy + (seed * 2.1 + t * (0.06 + i as f64 * 0.006)).cos() * spread * 0.65;
        let twinkle = 0.5 + 0.5 * (t * 1.6 + seed).sin();
        if twinkle < 0.35 {
            continue;
        }
        let r = 0.6 + twinkle * 0.9 * motion.glow;
        ctx.draw(&Circle {
            x: fx,
            y: fy,
            radius: r,
            color: blend_color(bg, particle, twinkle * 0.85 * motion.glow),
        });
    }
}

fn draw_idle_marker(
    ctx: &mut ratatui::widgets::canvas::Context,
    mx: f64,
    my: f64,
    t: f64,
    glow: Color,
    core: Color,
) {
    let pulse = 1.0 + 0.06 * (t * 0.7).sin();
    draw_soft_disc(ctx, mx, my, 4.5 * pulse, glow, glow, 5);
    ctx.draw(&Circle {
        x: mx,
        y: my,
        radius: 2.2,
        color: core,
    });
}

fn draw_task_constellation(
    ctx: &mut ratatui::widgets::canvas::Context,
    geom: (f64, f64, f64, f64),
    motion: SceneMotion,
    count: u32,
    active_idx: Option<u32>,
    colors: (Color, Color, Color),
) {
    let (cx, cy, base, t) = geom;
    let (task, task_dim, bg) = colors;
    let n = (count as usize).clamp(1, 12);
    let mut points = [(0.0, 0.0, 0); 12];

    for (i, pt) in points.iter_mut().enumerate().take(n) {
        let frac = if n == 1 {
            0.5
        } else {
            i as f64 / (n - 1) as f64
        };
        let angle = PI * 1.08 + PI * 0.84 * frac;
        let wobble = (t * 0.35 + i as f64 * 1.3).sin() * 1.8;
        let dist = base * (1.08 + 0.06 * (i as f64 * 0.5).sin()) + wobble;
        let px = cx + angle.cos() * dist;
        let py = cy + angle.sin() * dist * 0.55 - base * 0.08;
        *pt = (px, py, i);
    }

    if n > 1 {
        let mut lines = [(0.0, 0.0); 100];
        let mut l_idx = 0;
        for w in points[..n].windows(2) {
            let (x0, y0, _) = w[0];
            let (x1, y1, _) = w[1];
            let steps = 6;
            for s in 0..=steps {
                let f = s as f64 / steps as f64;
                if l_idx < lines.len() {
                    lines[l_idx] = (lerp(x0, x1, f), lerp(y0, y1, f));
                    l_idx += 1;
                }
            }
        }
        ctx.draw(&Points {
            coords: &lines[..l_idx],
            color: blend_color(bg, task_dim, 0.35),
        });
    }

    for &(px, py, i) in &points[..n] {
        let active = active_idx == Some(i as u32);
        let tw = if active {
            0.75 + 0.25 * (t * 2.2).sin()
        } else {
            0.3 + 0.2 * (t * 0.9 + i as f64).sin().max(0.0)
        };
        let color = blend_color(task_dim, task, tw * motion.glow);
        let r = if active {
            2.0 + 0.4 * (t * 2.5).sin()
        } else {
            1.1
        };
        if active {
            draw_soft_disc(ctx, px, py, r * 2.2, color, bg, 4);
        }
        ctx.draw(&Circle {
            x: px,
            y: py,
            radius: r,
            color,
        });
    }

    if count > 12 {
        draw_ring(
            ctx,
            cx,
            cy - base * 0.05,
            base * 1.28,
            blend_color(bg, task_dim, 0.5),
        );
    }
}

fn draw_task_orbit(
    ctx: &mut ratatui::widgets::canvas::Context,
    geom: (f64, f64, f64, f64),
    motion: SceneMotion,
    count: u32,
    active_idx: Option<u32>,
    colors: (Color, Color, Color),
) {
    let (cx, cy, base, t) = geom;
    let (task, task_dim, bg) = colors;
    let n = (count as usize).clamp(1, 12);
    let orbit = base * 1.2;
    let spin = t * motion.speed * 0.22;

    draw_ring(ctx, cx, cy, orbit, blend_color(bg, task_dim, 0.22));

    for i in 0..n {
        let angle = spin + 2.0 * PI * i as f64 / n as f64;
        let active = active_idx == Some(i as u32);
        let dist = if active { orbit * 0.92 } else { orbit };
        let wobble = (t * 0.4 + i as f64).sin() * 0.8;
        let px = cx + angle.cos() * (dist + wobble);
        let py = cy + angle.sin() * (dist + wobble) * 0.88;
        let tw = if active {
            0.8 + 0.2 * (t * 2.0).sin()
        } else {
            0.35 + 0.15 * (t * 0.8 + i as f64).sin().max(0.0)
        };
        let color = blend_color(task_dim, task, tw * motion.glow);
        let r = if active {
            1.9 + 0.35 * (t * 2.2).sin()
        } else {
            1.05
        };
        if active {
            let mut tether = [(0.0, 0.0); 10];
            for s in 1..=10 {
                let f = s as f64 / 10.0 * 0.55;
                tether[s - 1] = (
                    cx + angle.cos() * dist * f,
                    cy + angle.sin() * dist * f * 0.88,
                );
            }
            ctx.draw(&Points {
                coords: &tether,
                color: blend_color(bg, task_dim, 0.4),
            });
            draw_soft_disc(ctx, px, py, r * 2.0, color, bg, 4);
        }
        ctx.draw(&Circle {
            x: px,
            y: py,
            radius: r,
            color,
        });
    }

    if count > 12 {
        let pulse = 1.0 + 0.04 * (t * 2.0).sin();
        draw_ring(
            ctx,
            cx,
            cy,
            orbit * 1.1 * pulse,
            blend_color(bg, task, 0.35),
        );
    }
}

#[derive(Clone, Copy)]
enum SessionArc {
    Top,
    Bottom,
}

fn draw_soft_progress_wreath(
    ctx: &mut ratatui::widgets::canvas::Context,
    geom: (f64, f64, f64, f64),
    remaining: f64,
    colors: (Color, Color),
    timer_state: TimerState,
    compact: bool,
) {
    let (cx, cy, base, t) = geom;
    let (track, mode) = colors;
    let dots = if compact { 48 } else { 72 };
    for i in 0..dots {
        let frac = i as f64 / dots as f64;
        let a = -PI / 2.0 + 2.0 * PI * frac;
        let px = cx + a.cos() * base;
        let py = cy + a.sin() * base;
        let filled = frac <= remaining + 0.001;
        let pulse = if filled && timer_state == TimerState::Running {
            1.0 + 0.15 * (t * 3.0 + frac * 12.0).sin()
        } else {
            1.0
        };
        let color = if filled {
            if timer_state == TimerState::Paused {
                blend_color(mode, track, 0.5)
            } else {
                mode
            }
        } else {
            track
        };
        ctx.draw(&Circle {
            x: px,
            y: py,
            radius: if filled {
                if compact {
                    0.55 * pulse
                } else {
                    0.75 * pulse
                }
            } else if compact {
                0.35
            } else {
                0.45
            },
            color,
        });
    }
}

fn draw_timer_orb(
    ctx: &mut ratatui::widgets::canvas::Context,
    geom: (f64, f64, f64, f64),
    motion: SceneMotion,
    colors: (Color, Color, Color, Color),
    timer_state: TimerState,
    timer_mode: TimerMode,
    compact: bool,
) {
    let (cx, cy, base, breath) = geom;
    let (bg, glow, core, mode) = colors;
    let on_break = timer_mode.is_break();
    let warm = if on_break {
        blend_color(mode, core, 0.45)
    } else {
        blend_color(mode, glow, 0.35)
    };
    let scale = 0.9 + 0.1 * breath;
    let intensity = motion.glow;

    let layers: [(f64, Color, f64); 5] = if compact {
        [
            (0.95, blend_color(bg, glow, 0.22 * intensity), 0.35),
            (0.68, blend_color(bg, warm, 0.5 * intensity), 0.6),
            (0.38, blend_color(glow, core, 0.7 * intensity), 0.85),
            (0.0, bg, 0.0),
            (0.22, blend_color(core, mode, 0.45), 0.95),
        ]
    } else {
        [
            (1.18, blend_color(bg, glow, 0.12 * intensity), 0.15),
            (0.95, blend_color(bg, glow, 0.28 * intensity), 0.35),
            (0.72, blend_color(bg, warm, 0.55 * intensity), 0.55),
            (0.48, blend_color(glow, core, 0.65 * intensity), 0.75),
            (0.22, blend_color(core, mode, 0.4), 0.95),
        ]
    };

    for (frac, color, alpha) in layers {
        if frac <= 0.001 || alpha <= 0.001 {
            continue;
        }
        let r = base * frac * scale;
        let rings = if compact {
            (5.0 * alpha) as usize
        } else {
            (8.0 * alpha) as usize
        };
        draw_soft_disc(ctx, cx, cy, r, color, bg, rings.max(3));
    }

    if timer_state == TimerState::Running {
        let halo_r = base * (1.02 + 0.04 * breath);
        draw_ring(ctx, cx, cy, halo_r, blend_color(bg, warm, 0.4 * intensity));
    }
}

fn draw_soft_disc(
    ctx: &mut ratatui::widgets::canvas::Context,
    cx: f64,
    cy: f64,
    r: f64,
    color: Color,
    bg: Color,
    rings: usize,
) {
    let rings = rings.max(3);
    for i in 1..=rings {
        let frac = i as f64 / rings as f64;
        let rr = r * frac;
        let mix = frac * frac;
        draw_ring(ctx, cx, cy, rr, blend_color(bg, color, mix));
    }
}

fn draw_task_fill(
    ctx: &mut ratatui::widgets::canvas::Context,
    cx: f64,
    cy: f64,
    max_r: f64,
    progress: f64,
    task: Color,
    bg: Color,
) {
    let r = max_r * progress.clamp(0.0, 1.0);
    if r > 0.5 {
        draw_soft_disc(ctx, cx, cy, r, task, bg, 6);
    }
}

fn draw_session_stars(
    ctx: &mut ratatui::widgets::canvas::Context,
    geom: (f64, f64, f64, f64),
    sessions_done: u32,
    sessions_total: u32,
    colors: (Color, Color, Color),
    arc: SessionArc,
) {
    let (cx, cy, base, t) = geom;
    let (session_on, session_off, _bg) = colors;
    let orbit = base * 1.32;
    let span = PI * 0.55;
    let start = match arc {
        SessionArc::Top => PI / 2.0 - span / 2.0,
        SessionArc::Bottom => -PI / 2.0 - span / 2.0,
    };
    for i in 0..sessions_total {
        let frac = if sessions_total == 1 {
            0.5
        } else {
            i as f64 / (sessions_total - 1) as f64
        };
        let a = start + frac * span;
        let tw = if i < sessions_done {
            0.8 + 0.2 * (t * 2.0 + i as f64).sin()
        } else {
            0.4
        };
        let color = if i < sessions_done {
            blend_color(session_off, session_on, tw)
        } else {
            session_off
        };
        ctx.draw(&Circle {
            x: cx + a.cos() * orbit,
            y: cy + a.sin() * orbit,
            radius: if i < sessions_done { 1.5 * tw } else { 1.0 },
            color,
        });
    }
}

fn draw_soft_pause(
    ctx: &mut ratatui::widgets::canvas::Context,
    cx: f64,
    cy: f64,
    r: f64,
    color: Color,
) {
    for side in [-1.0_f64, 1.0] {
        ctx.draw(&Circle {
            x: cx + side * r * 0.55,
            y: cy,
            radius: r * 0.35,
            color: blend_color(color, color, 0.7),
        });
    }
}

fn draw_completion_shimmer(
    ctx: &mut ratatui::widgets::canvas::Context,
    cx: f64,
    cy: f64,
    base: f64,
    t: f64,
    mode: Color,
    glow: Color,
) {
    let pulse = 1.0 + 0.08 * (t * 2.8).sin();
    draw_ring(
        ctx,
        cx,
        cy,
        base * 1.22 * pulse,
        blend_color(mode, glow, 0.65),
    );
    for i in 0..8 {
        let a = t * 0.5 + i as f64 * PI / 4.0;
        let dist = base * (1.05 + 0.06 * (t * 3.0 + i as f64).sin());
        ctx.draw(&Circle {
            x: cx + a.cos() * dist,
            y: cy + a.sin() * dist,
            radius: 0.9,
            color: glow,
        });
    }
}

pub fn draw_timer_canvas(
    f: &mut Frame,
    area: Rect,
    timer: &Timer,
    style: &TimerCanvasStyle,
    options: &TimerCanvasOptions,
) {
    let scene = SceneStyle {
        mode: style.progress,
        track: style.track,
        task: style.task_progress,
        task_dim: style.task_track,
        bg: style.progress_dim,
        bg_mid: style.track,
        bg_light: style.track,
        wave: style.progress,
        core: style.cap,
        glow: style.progress,
        particle: style.dim,
        text: style.text,
        session_on: style.task_progress,
        session_off: style.dim,
    };
    draw_scene_canvas(
        f,
        area,
        timer,
        &scene,
        &SceneOptions {
            task_progress: options.task_progress,
            pending_tasks: if options.task_progress.is_some() {
                1
            } else {
                0
            },
            active_task_index: if options.task_progress.is_some() {
                Some(0)
            } else {
                None
            },
            sessions_done: 0,
            sessions_total: 0,
            layout: SceneLayout::Zen,
        },
    );
}

// ── geometry helpers ─────────────────────────────────────────────────────────

fn draw_ring(ctx: &mut ratatui::widgets::canvas::Context, cx: f64, cy: f64, r: f64, color: Color) {
    let mut coords = [(0.0, 0.0); ARC_STEPS];
    for (i, coord) in coords.iter_mut().enumerate().take(ARC_STEPS) {
        let a = arc_angle(i);
        *coord = (cx + a.cos() * r, cy + a.sin() * r);
    }
    ctx.draw(&Points {
        coords: &coords,
        color,
    });
}

fn arc_angle(i: usize) -> f64 {
    -PI / 2.0 + 2.0 * PI * (i as f64 / ARC_STEPS as f64)
}

fn smoothstep(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn blend_color(a: Color, b: Color, t: f64) -> Color {
    let t = t.clamp(0.0, 1.0);
    let (ar, ag, ab) = color_rgb(a);
    let (br, bg, bb) = color_rgb(b);
    Color::Rgb(
        lerp(ar, br, t) as u8,
        lerp(ag, bg, t) as u8,
        lerp(ab, bb, t) as u8,
    )
}

fn color_rgb(c: Color) -> (f64, f64, f64) {
    match c {
        Color::Rgb(r, g, b) => (r as f64, g as f64, b as f64),
        Color::Black => (0.0, 0.0, 0.0),
        Color::White => (255.0, 255.0, 255.0),
        Color::Red => (255.0, 0.0, 0.0),
        Color::Green => (0.0, 255.0, 0.0),
        Color::Blue => (0.0, 0.0, 255.0),
        Color::Yellow => (255.0, 255.0, 0.0),
        Color::Cyan => (0.0, 255.0, 255.0),
        Color::Magenta => (255.0, 0.0, 255.0),
        Color::Gray => (128.0, 128.0, 128.0),
        Color::DarkGray => (64.0, 64.0, 64.0),
        Color::LightRed => (255.0, 128.0, 128.0),
        Color::LightGreen => (128.0, 255.0, 128.0),
        Color::LightBlue => (128.0, 128.0, 255.0),
        Color::LightYellow => (255.0, 255.0, 128.0),
        Color::LightMagenta => (255.0, 128.0, 255.0),
        Color::LightCyan => (128.0, 255.0, 255.0),
        Color::Indexed(i) => {
            let v = (i as f64 / 255.0) * 255.0;
            (v, v, v)
        }
        Color::Reset => (200.0, 200.0, 200.0),
    }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn time_s() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64 / 1000.0)
        .unwrap_or(0.0)
}

fn canvas_marker(area: Rect) -> Marker {
    // Braille is 2× finer — use it whenever the panel is wide enough.
    if area.width >= 20 {
        Marker::Braille
    } else {
        Marker::HalfBlock
    }
}

fn square_bounds(area: Rect) -> ([f64; 2], [f64; 2]) {
    let w = area.width as f64;
    let h = area.height as f64 * 2.0;
    if w >= h {
        let pad = (w - h) / 2.0;
        ([pad, pad + h], [0.0, h])
    } else {
        let pad = (h - w) / 2.0;
        ([0.0, w], [pad, pad + w])
    }
}

fn center(xb: [f64; 2], yb: [f64; 2]) -> (f64, f64) {
    ((xb[0] + xb[1]) / 2.0, (yb[0] + yb[1]) / 2.0)
}

pub fn session_dots(done_in_cycle: u32, cycle_length: u32, in_focus: bool) -> String {
    let cycle = cycle_length.max(1);
    let done = done_in_cycle % cycle;
    (1..=cycle)
        .map(|i| {
            if i <= done {
                '●'
            } else if in_focus && i == done + 1 {
                '◉'
            } else {
                '○'
            }
        })
        .collect()
}

pub fn format_time_stack(timer: &Timer) -> (String, String, String) {
    let (main, tenths) = timer.format_remaining_parts();
    let mode = timer.mode.label().to_string();
    (main, tenths, mode)
}

pub struct SimpleTimerStyle {
    pub track: Color,
    pub fill: Color,
    pub fill_dim: Color,
    pub task: Color,
    pub text: Color,
    pub dim: Color,
}

const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

pub fn draw_simple_timer(
    f: &mut Frame,
    area: Rect,
    timer: &Timer,
    style: &SimpleTimerStyle,
    task_progress: Option<f64>,
) {
    if area.height < 3 || area.width < 8 {
        return;
    }

    let remaining = (1.0 - timer.progress()).clamp(0.0, 1.0);
    let paused = timer.state == TimerState::Paused;
    let running = timer.state == TimerState::Running;
    let finished = timer.state == TimerState::Finished;

    let fill_color = if paused { style.fill_dim } else { style.fill };

    let layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(1),
            ratatui::layout::Constraint::Length(1),
            ratatui::layout::Constraint::Min(1),
        ])
        .split(area);

    let segments = area.width.saturating_sub(2) as usize;
    let filled = (remaining * segments as f64).round() as usize;
    let bar: String = (0..segments)
        .map(|i| if i < filled { '█' } else { '░' })
        .collect();

    let status_glyph = if finished {
        '✓'
    } else if paused {
        '❚'
    } else if running {
        let idx = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() / 80)
            .unwrap_or(0) as usize)
            % SPINNER.len();
        SPINNER[idx]
    } else {
        '○'
    };

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{} ", status_glyph),
                ratatui::style::Style::default().fg(fill_color),
            ),
            Span::styled(bar, ratatui::style::Style::default().fg(fill_color)),
            Span::styled(
                format!(" {:>3}%", (remaining * 100.0) as u32),
                ratatui::style::Style::default().fg(style.dim),
            ),
        ])),
        layout[0],
    );

    if let Some(tp) = task_progress {
        let task_segments = area.width.saturating_sub(6) as usize;
        let task_filled = (tp.clamp(0.0, 1.0) * task_segments as f64).round() as usize;
        let task_bar: String = (0..task_segments)
            .map(|i| if i < task_filled { '▰' } else { '▱' })
            .collect();
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("task ", ratatui::style::Style::default().fg(style.dim)),
                Span::styled(task_bar, ratatui::style::Style::default().fg(style.task)),
            ])),
            layout[1],
        );
    } else {
        f.render_widget(
            Paragraph::new(Span::styled(
                "─ no active task ─",
                ratatui::style::Style::default().fg(style.dim),
            ))
            .alignment(Alignment::Center),
            layout[1],
        );
    }
}
