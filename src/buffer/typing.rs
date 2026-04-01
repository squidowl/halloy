use std::f32::consts::PI;
use std::time::{Duration, Instant};

use data::config::buffer::Style;
use data::history::filter::FilterChain;
use data::isupport::CaseMap;
use data::user::Nick;
use data::{Config, Server, User, target};
use iced::widget::{column, container, row};
use iced::{Color, padding};

use crate::widget::{self, Element};
use crate::{Theme, font, icon, theme};

const DOT_COUNT: usize = 3;
const DOT_BASE_OPACITY: f32 = 0.35;
const DOT_PEAK_OPACITY: f32 = 1.0;
const DOT_DURATION: Duration = Duration::from_millis(520);
#[derive(Debug, Clone, Copy)]
pub struct Animation {
    started_at: Instant,
    last_updated_at: Instant,
}

impl Animation {
    fn new(now: Instant) -> Self {
        Self {
            started_at: now,
            last_updated_at: now,
        }
    }

    fn update(&mut self, now: Instant) {
        self.last_updated_at = now;
    }

    fn opacities(&self) -> [f32; DOT_COUNT] {
        let elapsed = self
            .last_updated_at
            .saturating_duration_since(self.started_at);

        opacities(elapsed)
    }
}

pub fn typing_font_size(config: &Config) -> f32 {
    config
        .buffer
        .typing
        .font_size
        .or(config.font.size)
        .map_or(theme::TEXT_SIZE, f32::from)
}

pub fn typing_line_height(config: &Config) -> f32 {
    theme::line_height(&config.font)
        .to_absolute(typing_font_size(config).into())
        .0
}

pub fn reserved_bottom_padding(
    has_typing_text: bool,
    style: Style,
    config: &Config,
) -> f32 {
    if has_typing_text || matches!(style, Style::Padded) {
        // Do not include line-spacing in the reserved height
        typing_line_height(config) + 2.0
    } else {
        0.0
    }
}

pub fn show_row(
    show_typing: bool,
    style: Style,
    has_typing_text: bool,
) -> bool {
    show_typing
        && match style {
            Style::Padded => true,
            Style::Popped => has_typing_text,
        }
}

pub fn view<'a, Message: 'a>(
    typing: Option<String>,
    animation: Option<&Animation>,
    font_size: f32,
    line_spacing: u32,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let typing: Element<'a, Message> = match typing {
        Some(text) => {
            let secondary_font =
                theme::font_style::secondary(theme).map(font::get);
            let dot_color = theme.styles().text.secondary.color;
            let dot_opacities = animation
                .map_or([DOT_BASE_OPACITY; DOT_COUNT], Animation::opacities);
            let dot_font_size = font_size * 0.33;
            container(
                row![
                    widget::text(text)
                        .size(font_size)
                        .style(theme::text::secondary)
                        .font_maybe(secondary_font.clone()),
                    row((0..DOT_COUNT).zip(dot_opacities).map(
                        |(_, opacity)| {
                            let color = Color {
                                a: dot_color.a * opacity,
                                ..dot_color
                            };

                            icon::dot().size(dot_font_size).color(color).into()
                        }
                    ),)
                    .spacing(4)
                ]
                .align_y(iced::Alignment::Center)
                .spacing(0),
            )
        }
        .padding(padding::left(14).top(2 + line_spacing).right(14))
        .align_y(iced::alignment::Vertical::Bottom)
        .style(theme::container::typing)
        .into(),
        None => column![].into(),
    };

    typing
}

pub fn visible_nicks(
    nicks: &[String],
    channel: Option<&target::Channel>,
    server: &Server,
    filters: FilterChain<'_>,
    casemapping: CaseMap,
) -> Vec<String> {
    nicks
        .iter()
        .filter(|nick| {
            let user = User::from(Nick::from_str(nick, casemapping));

            !filters.filter_user(&user, channel, server)
        })
        .cloned()
        .collect()
}

pub fn typing_text(
    enabled: bool,
    supports_typing: bool,
    our_nick: Option<&str>,
    nicks: &[String],
    casemapping: CaseMap,
) -> Option<String> {
    if !enabled || !supports_typing {
        return None;
    }

    let filtered: Vec<_> = nicks
        .iter()
        .filter(|nick| {
            our_nick.is_none_or(|our| {
                casemapping.normalize(nick) != casemapping.normalize(our)
            })
        })
        .collect();

    match filtered.len() {
        0 => None,
        1 => Some(format!("{} is typing ", filtered[0])),
        2 => Some(format!("{} and {} are typing ", filtered[0], filtered[1])),
        _ => Some("Several people are typing ".to_string()),
    }
}

pub fn update(
    animation: &mut Option<Animation>,
    is_typing: bool,
    now: Instant,
) {
    match (animation.as_mut(), is_typing) {
        (Some(animation), true) => animation.update(now),
        (None, true) => *animation = Some(Animation::new(now)),
        (_, false) => *animation = None,
    }
}

fn opacities(elapsed: Duration) -> [f32; DOT_COUNT] {
    let cycle = DOT_DURATION.saturating_mul(DOT_COUNT as u32);
    let elapsed = elapsed.as_secs_f32().rem_euclid(cycle.as_secs_f32());
    let dot_duration = DOT_DURATION.as_secs_f32();

    std::array::from_fn(|index| {
        let start = index as f32 * dot_duration;
        let end = start + dot_duration;

        if (start..end).contains(&elapsed) {
            let progress = (elapsed - start) / dot_duration;
            let pulse = pulse(progress);

            DOT_BASE_OPACITY + pulse * (DOT_PEAK_OPACITY - DOT_BASE_OPACITY)
        } else {
            DOT_BASE_OPACITY
        }
    })
}

fn pulse(progress: f32) -> f32 {
    let fade = if progress <= 0.5 {
        ease_in_out_sine(progress * 2.0)
    } else {
        ease_in_out_sine((1.0 - progress) * 2.0)
    };

    fade.clamp(0.0, 1.0)
}

fn ease_in_out_sine(progress: f32) -> f32 {
    0.5 * (1.0 - (PI * progress.clamp(0.0, 1.0)).cos())
}
