#![allow(clippy::too_many_arguments)]
use std::f64::consts::PI;

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Color;
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::canvas::{Canvas, Points};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;

use crate::model::{TimerMode, TimerState};
use crate::timer::Timer;

const ARC_STEPS: usize = 360;
const PARTICLE_COUNT: usize = 7;

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
    pub animated: bool,
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
    let motion = if options.animated {
        scene_motion(timer)
    } else {
        SceneMotion {
            breath: 0.5,
            speed: 0.0,
            scale: 1.0,
            glow: 1.0,
        }
    };
    let marker = canvas_marker(area);
    let (xb, yb) = square_bounds(area, marker);
    let t = if options.animated { time_s() } else { 0.0 };
    let zen = options.layout == SceneLayout::Zen;
    let compact = !zen;
    let extent = canvas_extent(xb, yb);
    let base_r = fit_base_r(extent, options.layout);

    let bg = style.bg;
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
        // Canvas defaults to `Color::Reset` and unconditionally repaints its whole area
        // with it, which erases the theme background and lets the terminal's own colour
        // show through. On a light terminal running a dark theme that left the scene
        // sitting in a pale rectangle while every panel around it stayed dark.
        .background_color(bg)
        .x_bounds(xb)
        .y_bounds(yb)
        .paint(move |ctx| {
            let (cx, cy) = center(xb, yb);
            let breath = motion.breath;
            let base = base_r * motion.scale * (0.86 + 0.14 * remaining);

            // Ambience is zen-only: the dashboard band is about five rows tall, and drifting
            // stars there just speckle the strip.
            //
            // What used to be here as well was a wide background wash ellipse and a pair of
            // horizontal wave lines. Both were sampled at a fixed point count, so on a wide
            // terminal their samples ended up cells apart and they degraded into dotted
            // streaks running straight through the orb — with the wreath and the contours
            // that made eight nested outlines competing for the same space. The orb is the
            // subject; the stars are the only atmosphere it needs.
            if zen {
                draw_star_halo(
                    ctx,
                    (cx, cy, base, t),
                    motion,
                    (particle, bg),
                    if pending == 0 {
                        PARTICLE_COUNT + 4
                    } else {
                        PARTICLE_COUNT
                    },
                );
            }

            // The dashboard band is about five rows tall. Everything the constellation and
            // the orbit encode — how many tasks are open, which one is active — is already
            // spelled out in the text rows directly under the canvas, and at this height
            // the extra dots land on top of the wreath and fill the orb in. Zen has the
            // room for them; the band does not.
            if zen {
                if pending == 0 {
                    draw_idle_marker(ctx, cx + base * 0.95, cy - base * 0.72, t, glow, core);
                } else {
                    draw_task_constellation(
                        ctx,
                        (cx, cy, base, t),
                        motion,
                        pending,
                        active_idx,
                        (task, task_dim, bg),
                    );
                }
            }

            draw_soft_progress_wreath(
                ctx,
                (
                    cx,
                    cy,
                    base * if zen {
                        WREATH_RADIUS_ZEN
                    } else {
                        WREATH_RADIUS_DASH
                    },
                    t,
                ),
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

            // Same reasoning: `◉ ○ ○ ○` is printed under the canvas either way, so the
            // arc of stars is duplicate information that only zen has the height for.
            if zen && sessions_total > 0 {
                draw_session_stars(
                    ctx,
                    (cx, cy, base, t),
                    sessions_done,
                    sessions_total,
                    (session_on, session_off, bg),
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
        SceneLayout::Zen => 0.32,
        SceneLayout::Dashboard => 0.40,
    };
    (extent * frac).clamp(3.5, 36.0)
}

/// Golden angle — spacing successive stars by it keeps them from clumping.
const GOLDEN_ANGLE: f64 = 2.399_963_229_728_653;

/// Slow-drifting stars in a ring just outside the wreath.
///
/// These used to be scattered across a box `1.45 x base` wide and `0.65` of that tall. Two
/// problems: the vertical squash was compensating for the old stretched bounds, so with
/// square units it now reads as a deliberate oval; and a rectangular scatter puts stars in
/// the corners of the panel where they look like stray pixels rather than a halo. Keeping
/// them in an annulus around the orb makes them part of the same object.
fn draw_star_halo(
    ctx: &mut ratatui::widgets::canvas::Context,
    geom: (f64, f64, f64, f64),
    motion: SceneMotion,
    colors: (Color, Color),
    count: usize,
) {
    let (cx, cy, base, t) = geom;
    let (particle, _bg) = colors;
    let count = count.min(PARTICLE_COUNT + 4);
    let inner = base * 1.55;
    let band = base * 0.45;

    let mut star_pts = [(0.0, 0.0); PARTICLE_COUNT + 4];
    let mut star_len = 0;

    for i in 0..count {
        let seed = i as f64 * GOLDEN_ANGLE;
        let twinkle = 0.5 + 0.5 * (t * 1.6 + seed).sin();
        if twinkle < 0.30 {
            continue;
        }
        let a = seed + t * motion.speed * (0.012 + i as f64 * 0.0015);
        let r = inner + band * (0.5 + 0.5 * (seed * 3.1).sin());
        let px = cx + a.cos() * r;
        let py = cy + a.sin() * r;
        star_pts[star_len] = (px, py);
        star_len += 1;
    }

    ctx.draw(&Points {
        coords: &star_pts[..star_len],
        color: particle,
    });
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
    draw_dot(ctx, mx, my, 2.2, core);
}

/// Open tasks, as markers spaced along an arc outside the wreath.
///
/// This used to join the markers with straight chords and give the active one a nested
/// "soft disc". Both cut across the orb: the chords ran straight through the rings, and the
/// disc's rings landed a dot apart and hashed into a smudge. Following the arc keeps the
/// markers reading as part of the circle rather than as marks thrown over it.
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
    let n = (count as usize).clamp(1, MAX_CONSTELLATION);
    // Outside the wreath, and outside the halo ring the active marker gets, so nothing
    // here can merge with the ring itself.
    let orbit = base * 1.45;
    // Anchored to the bottom of the orb, mirroring the session stars along the top. It is
    // deliberately not rotated: `t` is absolute wall-clock seconds, so any bare `t * k`
    // rotation puts the arc at an arbitrary angle on every launch.
    let span = PI * 0.62;
    let start = -PI / 2.0 - span / 2.0;

    for i in 0..n {
        let frac = if n == 1 {
            0.5
        } else {
            i as f64 / (n - 1) as f64
        };
        let a = start + span * frac;
        let px = cx + a.cos() * orbit;
        let py = cy + a.sin() * orbit;
        let active = active_idx == Some(i as u32);
        let tw = if active {
            0.75 + 0.25 * (t * 2.2).sin()
        } else {
            0.3 + 0.2 * (t * 0.9 + i as f64).sin().max(0.0)
        };
        let color = blend_color(task_dim, task, tw * motion.glow);
        draw_dot(ctx, px, py, if active { 1.9 } else { 1.0 }, color);
        if active {
            // Clear of the marker, so the two do not merge into one blob.
            draw_ring(ctx, px, py, MIN_RING_GAP, blend_color(bg, color, 0.5));
        }
    }

    if count as usize > MAX_CONSTELLATION {
        draw_ring(ctx, cx, cy, orbit * 1.1, blend_color(bg, task_dim, 0.4));
    }
}

/// Past a dozen the markers stop being countable; an extra ring stands in for the rest.
const MAX_CONSTELLATION: usize = 12;

/// Angle of a point `frac` of the way around the wreath: clockwise from twelve o'clock.
fn wreath_angle(frac: f64) -> f64 {
    PI / 2.0 - 2.0 * PI * frac
}

/// Beads to lay around the wreath at a given radius.
///
/// A fixed count only works at one size. 48 beads around the short dashboard orb put them
/// half a unit apart while each was more than a unit across, so they fused into a solid
/// band — the orb showed up as a filled blob rather than a ring. Spacing them by screen
/// distance instead keeps them reading as beads at every panel size.
fn wreath_bead_count(base: f64, compact: bool) -> usize {
    // One canvas unit is half a braille dot, so the circumference in dots is 4πr.
    let circumference_dots = 4.0 * PI * base.abs();
    let spacing = if compact { 3.0 } else { 3.6 };
    // The zen ring is big enough to need well over the old fixed 72 before the beads stop
    // touching; the cap is only here so an absurd radius cannot run away with the frame.
    let cap = if compact { 48 } else { 192 };
    ((circumference_dots / spacing).round() as usize).clamp(8, cap)
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
    let dots = wreath_bead_count(base, compact);

    // Batched into two point sets rather than one shape per dot. This is the hot path of
    // the whole scene: as individual `Circle`s it was 72 × 360 trig operations a frame.
    let mut filled_pts: Vec<(f64, f64)> = Vec::with_capacity(dots * MAX_DOT_PTS / 2);
    let mut track_pts: Vec<(f64, f64)> = Vec::with_capacity(dots * 8);
    let mut buf = [(0.0, 0.0); MAX_DOT_PTS];

    for i in 0..dots {
        let frac = i as f64 / dots as f64;
        // Twelve o'clock, depleting clockwise, the way every other progress ring reads.
        // Canvas y points up, so screen-clockwise is a *decreasing* angle from +PI/2.
        let a = wreath_angle(frac);
        let px = cx + a.cos() * base;
        let py = cy + a.sin() * base;

        if frac <= remaining + 0.001 {
            let pulse = if timer_state == TimerState::Running {
                1.0 + 0.15 * (t * 3.0 + frac * 12.0).sin()
            } else {
                1.0
            };
            let r = if compact { 0.55 } else { 0.75 } * pulse;
            let n = fill_dot(&mut buf, px, py, r);
            filled_pts.extend_from_slice(&buf[..n]);
        } else {
            let r = if compact { 0.35 } else { 0.45 };
            let n = fill_dot(&mut buf, px, py, r);
            track_pts.extend_from_slice(&buf[..n]);
        }
    }

    let filled_color = if timer_state == TimerState::Paused {
        blend_color(mode, track, 0.5)
    } else {
        mode
    };

    ctx.draw(&Points {
        coords: &track_pts,
        color: track,
    });
    ctx.draw(&Points {
        coords: &filled_pts,
        color: filled_color,
    });

    // A brighter head at the boundary. The ring is otherwise uniform, so where the time
    // has actually got to is hard to find at a glance — the cap gives the eye a target
    // and makes the depletion direction obvious.
    if remaining > 0.001 && remaining < 0.999 {
        let a = wreath_angle(remaining);
        let hx = cx + a.cos() * base;
        let hy = cy + a.sin() * base;
        let pulse = if timer_state == TimerState::Running {
            1.0 + 0.2 * (t * 2.5).sin()
        } else {
            1.0
        };
        let r = if compact { 0.9 } else { 1.2 } * pulse;
        draw_dot(ctx, hx, hy, r * 1.7, blend_color(track, mode, 0.55));
        draw_dot(ctx, hx, hy, r, mode);
    }
}

/// Smallest gap between contours, in canvas units, that still reads as two rings.
const MIN_RING_GAP: f64 = 3.0;

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
    let intensity = motion.glow;

    // In Zen mode, draw a subtle outer rim aura that gracefully frames the center plate
    if !compact {
        let halo_r = base * 0.94;
        draw_ring(ctx, cx, cy, halo_r, blend_color(bg, warm, 0.22 * intensity));
    }

    if timer_state == TimerState::Running {
        let halo_r = base * (1.06 + 0.05 * breath);
        draw_ring(ctx, cx, cy, halo_r, blend_color(bg, warm, 0.32 * intensity));
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
    // Only as many rings as the radius can space out. Braille dots are 1-bit and one colour
    // per cell, so rings closer than `MIN_RING_GAP` interleave into a hatch instead of a
    // gradient — a small "soft" disc came out as a dense scribble. Below that, one filled
    // dot is both cheaper and closer to the intent.
    let rings = rings.min(((r.abs() / MIN_RING_GAP).floor() as usize).max(1));
    if rings < 2 {
        draw_dot(ctx, cx, cy, r, blend_color(bg, color, 0.85));
        return;
    }
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
) {
    let (cx, cy, base, t) = geom;
    let (session_on, session_off, _bg) = colors;
    let orbit = base * 1.32;
    let span = PI * 0.55;
    // Along the top of the orb. The bottom arc was the dashboard variant, and the dashboard
    // no longer draws stars at all — `◉ ○ ○ ○` is printed under the band instead.
    let start = PI / 2.0 - span / 2.0;
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
        draw_dot(
            ctx,
            cx + a.cos() * orbit,
            cy + a.sin() * orbit,
            if i < sessions_done { 1.5 * tw } else { 1.0 },
            color,
        );
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
        draw_dot(ctx, cx + side * r * 0.55, cy, r * 0.35, color);
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
        draw_dot(ctx, cx + a.cos() * dist, cy + a.sin() * dist, 0.9, glow);
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
            animated: options.breathe,
        },
    );
}

// ── geometry helpers ─────────────────────────────────────────────────────────

/// Samples a ring at a density proportional to its circumference.
///
/// Every ring used to be walked at a fixed 360 steps. Most rings in the scene are small —
/// a radius-2 ring only covers about 25 distinct cells, so 360 samples did the same work
/// fourteen times over.
fn ring_steps(r: f64) -> usize {
    ((r.abs() * 2.0 * PI * 2.0).ceil() as usize).clamp(16, ARC_STEPS)
}

fn draw_ring(ctx: &mut ratatui::widgets::canvas::Context, cx: f64, cy: f64, r: f64, color: Color) {
    let steps = ring_steps(r);
    let mut coords = [(0.0, 0.0); ARC_STEPS];
    for (i, coord) in coords.iter_mut().enumerate().take(steps) {
        let a = -PI / 2.0 + 2.0 * PI * (i as f64 / steps as f64);
        *coord = (cx + a.cos() * r, cy + a.sin() * r);
    }
    ctx.draw(&Points {
        coords: &coords[..steps],
        color,
    });
}

/// Upper bound on the samples [`fill_dot`] writes.
const MAX_DOT_PTS: usize = 25;

/// Writes a small filled dot into `buf`, returning how many samples were used.
///
/// `ratatui`'s `Circle` walks 360 angles whatever its radius, so using it for the sub-unit
/// dots this scene is built from cost ~360 trig operations each. At these sizes a handful
/// of samples is visually identical.
fn fill_dot(buf: &mut [(f64, f64); MAX_DOT_PTS], x: f64, y: f64, r: f64) -> usize {
    buf[0] = (x, y);
    let mut n = 1;
    if r <= 0.35 {
        return n;
    }
    let ring = if r <= 0.8 { 6 } else { 10 };
    for i in 0..ring {
        let a = 2.0 * PI * i as f64 / ring as f64;
        buf[n] = (x + a.cos() * r, y + a.sin() * r);
        n += 1;
    }
    // Larger dots need an inner ring or they render as an outline rather than a disc.
    if r > 1.1 {
        for i in 0..ring {
            let a = 2.0 * PI * (i as f64 + 0.5) / ring as f64;
            buf[n] = (x + a.cos() * r * 0.55, y + a.sin() * r * 0.55);
            n += 1;
        }
    }
    n
}

fn draw_dot(ctx: &mut ratatui::widgets::canvas::Context, x: f64, y: f64, r: f64, color: Color) {
    let mut buf = [(0.0, 0.0); MAX_DOT_PTS];
    let n = fill_dot(&mut buf, x, y, r);
    ctx.draw(&Points {
        coords: &buf[..n],
        color,
    });
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

/// Dots ratatui renders per cell for a marker, horizontally and vertically.
fn marker_resolution(marker: Marker) -> (f64, f64) {
    match marker {
        Marker::Braille => (2.0, 4.0),
        Marker::HalfBlock => (1.0, 2.0),
        _ => (1.0, 1.0),
    }
}

/// Bounds whose units are square on screen.
///
/// The canvas maps the x range across `width * res_x` dots and the y range across
/// `height * res_y` dots. Making both ranges the same *number* — which is what this used
/// to do — does not make them the same *scale*, because braille packs 2 dots per column
/// but 4 per row. Circles came out stretched by the ratio: about 2.2x in zen and 10x in
/// the short dashboard strip, where they smeared into the speckle across the whole band.
///
/// The vertical unit is left as it was (one unit per two rows) so the radii tuned against
/// it still hold; only the width is corrected.
fn square_bounds(area: Rect, marker: Marker) -> ([f64; 2], [f64; 2]) {
    let (rx, ry) = marker_resolution(marker);
    let h = (area.height as f64 * 2.0).max(1.0);
    let dots_x = area.width as f64 * rx;
    let dots_y = (area.height as f64 * ry).max(1.0);
    let w = h * dots_x / dots_y;
    ([0.0, w], [0.0, h])
}

/// Widest text plate, in columns, that still sits inside the zen wreath.
///
/// The zen overlay clears a rectangle behind its text so the canvas cannot collide with the
/// digits. Sized from the text alone, that rectangle grew wider than the ring on mid-width
/// terminals and erased the ring's left and right extremes, leaving a top arc and a bottom
/// arc that looked accidentally chopped. Bounding the plate by the ring instead keeps the
/// circle whole and puts the text inside it, which is the composition the scene is for.
pub fn scene_plate_width(area: Rect) -> u16 {
    if area.width < 8 || area.height < 4 {
        return area.width;
    }
    let (xb, yb) = square_bounds(area, canvas_marker(area));
    let base = fit_base_r(canvas_extent(xb, yb), SceneLayout::Zen);
    // One canvas unit is one column across, so the wreath radius is already in columns.
    // 1.4x the radius is the chord that leaves room for the plate's own height.
    let inside = (base * WREATH_RADIUS_ZEN * 1.4) as u16;
    inside.clamp(MIN_PLATE_W.min(area.width), area.width)
}

/// Where the wreath sits relative to the orb radius.
const WREATH_RADIUS_ZEN: f64 = 1.18;
const WREATH_RADIUS_DASH: f64 = 1.12;
/// Below this the plate is unreadable, so a small terminal gets an overlapped ring instead.
const MIN_PLATE_W: u16 = 30;

fn center(xb: [f64; 2], yb: [f64; 2]) -> (f64, f64) {
    ((xb[0] + xb[1]) / 2.0, (yb[0] + yb[1]) / 2.0)
}

/// Renders the pomodoro cycle as `● ● ◉ ○` — done, done, current, upcoming.
///
/// `on_focus_cycle` marks the current slot. It is deliberately not tied to the timer
/// *running*: an idle timer is still sitting on a specific session of the cycle, and
/// without the marker every slot looked identical before the first start.
pub fn session_dots(done_in_cycle: u32, cycle_length: u32, on_focus_cycle: bool) -> String {
    let cycle = cycle_length.max(1);
    let done = done_in_cycle % cycle;
    let glyphs: Vec<String> = (1..=cycle)
        .map(|i| {
            if i <= done {
                '●'
            } else if on_focus_cycle && i == done + 1 {
                '◉'
            } else {
                '○'
            }
            .to_string()
        })
        .collect();
    glyphs.join(" ")
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;
    use ratatui::Terminal;

    /// One canvas unit must cover the same distance horizontally and vertically.
    ///
    /// The canvas spreads `x_bounds` over `width * res_x` dots and `y_bounds` over
    /// `height * res_y` dots. Braille packs 2 dots per column but 4 per row, so equal
    /// *ranges* are not equal *scales* — that mismatch is what stretched every circle.
    #[test]
    fn a_canvas_unit_is_the_same_size_on_both_axes() {
        for marker in [Marker::Braille, Marker::HalfBlock] {
            let (rx, ry) = marker_resolution(marker);
            for (w, h) in [(190u16, 44u16), (100, 30), (100, 5), (60, 12), (24, 4)] {
                let area = Rect::new(0, 0, w, h);
                let (xb, yb) = square_bounds(area, marker);

                let per_dot_x = (xb[1] - xb[0]) / (f64::from(w) * rx);
                let per_dot_y = (yb[1] - yb[0]) / (f64::from(h) * ry);
                assert!(
                    (per_dot_x - per_dot_y).abs() < 1e-9,
                    "{marker:?} {w}x{h}: {per_dot_x} units/dot across vs {per_dot_y} down",
                );
            }
        }
    }

    /// Braille dots are square on screen (half a cell wide, a quarter of a cell tall, and
    /// cells are about twice as tall as they are wide), so a circle drawn in canvas units
    /// must come out twice as many columns wide as it is rows tall.
    #[test]
    fn a_drawn_ring_is_round_on_screen() {
        let (w, h) = (100u16, 30u16);
        let area = Rect::new(0, 0, w, h);
        let (xb, yb) = square_bounds(area, Marker::Braille);
        let (cx, cy) = center(xb, yb);
        let r = 12.0;

        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| {
            let canvas = Canvas::default()
                .marker(Marker::Braille)
                .background_color(Color::Black)
                .x_bounds(xb)
                .y_bounds(yb)
                .paint(|ctx| draw_ring(ctx, cx, cy, r, Color::White));
            f.render_widget(canvas, area);
        })
        .unwrap();

        let buf = term.backend().buffer();
        let (mut x0, mut x1, mut y0, mut y1) = (u16::MAX, 0u16, u16::MAX, 0u16);
        for y in 0..h {
            for x in 0..w {
                if buf[(x, y)].symbol() != " " {
                    x0 = x0.min(x);
                    x1 = x1.max(x);
                    y0 = y0.min(y);
                    y1 = y1.max(y);
                }
            }
        }
        assert!(x1 >= x0, "nothing was drawn");

        let cols = f64::from(x1 - x0 + 1);
        let rows = f64::from(y1 - y0 + 1);
        // Two columns per row, within a cell of rounding on each axis.
        assert!(
            (cols / rows - 2.0).abs() < 0.2,
            "ring is {cols} cols x {rows} rows (ratio {:.2}, want 2.0)",
            cols / rows
        );
    }
}
