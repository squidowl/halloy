use data::{Config, Preview, preview};
use iced::widget::{column, container, text};
use iced::{ContentFit, Padding};

use super::Element;
use crate::widget::image;
use crate::{Theme, font, theme};

pub fn preview_card_parts<'a, M: 'a>(
    preview: &'a preview::Card,
    config: &'a Config,
    theme: &'a Theme,
) -> (Element<'a, M>, Element<'a, M>, Option<Element<'a, M>>) {
    let preview::Card {
        image: card_image,
        title,
        description,
        ..
    } = preview;

    let title = text(title)
        .shaping(text::Shaping::Advanced)
        .style(theme::text::primary)
        .font_maybe(theme::font_style::primary(theme).map(font::get))
        .into();

    let description = description
        .as_ref()
        .map(|description| {
            container(
                text(description)
                    .shaping(text::Shaping::Advanced)
                    .wrapping(text::Wrapping::WordOrGlyph)
                    .style(theme::text::secondary)
                    .font_maybe(
                        theme::font_style::secondary(theme).map(font::get),
                    ),
            )
            .clip(false)
            .max_height(config.preview.card.description_max_height)
        })
        .into();

    let image = config.preview.card.show_image.then_some(
        container(image::from_data(
            card_image,
            config.preview.card.round_image_corners,
            ContentFit::ScaleDown,
        ))
        .padding(Padding::default().top(8))
        .max_height(config.preview.card.image_max_height)
        .into(),
    );

    (title, description, image)
}

pub fn preview_content<'a, M: 'a>(
    preview: &'a Preview,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, M> {
    match preview {
        Preview::Card(preview) => {
            let (title, description, image) =
                preview_card_parts(preview, config, theme);

            container(
                column![title, description, image]
                    .spacing(8)
                    .max_width(config.preview.card.max_width),
            )
            .padding(8)
            .into()
        }

        Preview::Image(img) => container(image::from_data(
            img,
            config.preview.image.round_corners,
            ContentFit::ScaleDown,
        ))
        .max_width(config.preview.image.max_width)
        .max_height(config.preview.image.max_height)
        .into(),
    }
}
