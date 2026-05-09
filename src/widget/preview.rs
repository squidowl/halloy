use data::{Config, Image, Preview, preview};
use iced::widget::{column, container, image, text};
use iced::{ContentFit, Padding};

use super::Element;
use crate::{Theme, font, theme};

pub fn preview_content<'a, M: 'a>(
    preview: &'a Preview,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, M> {
    match preview {
        Preview::Card(preview::Card {
            image: Image { path, .. },
            title,
            description,
            ..
        }) => container(
            column![
                text(title)
                    .shaping(text::Shaping::Advanced)
                    .style(theme::text::primary)
                    .font_maybe(
                        theme::font_style::primary(theme).map(font::get)
                    ),
                description.as_ref().map(|description| {
                    container(
                        text(description)
                            .shaping(text::Shaping::Advanced)
                            .wrapping(text::Wrapping::WordOrGlyph)
                            .style(theme::text::secondary)
                            .font_maybe(
                                theme::font_style::secondary(theme)
                                    .map(font::get),
                            ),
                    )
                    .clip(false)
                    .max_height(config.preview.card.description_max_height)
                }),
                config.preview.card.show_image.then_some(
                    container(
                        image(path)
                            .border_radius(
                                if config.preview.card.round_image_corners {
                                    4
                                } else {
                                    0
                                }
                            )
                            .content_fit(ContentFit::ScaleDown),
                    )
                    .padding(Padding::default().top(8))
                    .max_height(config.preview.card.image_max_height),
                ),
            ]
            .spacing(8)
            .max_width(config.preview.card.max_width),
        )
        .padding(8)
        .into(),

        Preview::Image(Image { path, .. }) => container(
            image(path)
                .border_radius(if config.preview.image.round_corners {
                    4
                } else {
                    0
                })
                .content_fit(ContentFit::ScaleDown),
        )
        .max_width(config.preview.image.max_width)
        .max_height(config.preview.image.max_height)
        .into(),
    }
}
