use data::{Config, Preview, preview};
use iced::Length::Fit;
use iced::widget::{column, container, stack, text};
use iced::{ContentFit, Fill, Padding};

use super::Element;
use crate::widget::{animated_image, image};
use crate::{Theme, font, theme};

pub fn preview_card_parts<'a, M: 'a>(
    preview: &'a preview::Card,
    config: &'a Config,
    theme: &'a Theme,
    on_hover: Option<fn(animated_image::hover::Request) -> M>,
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
            .height(Fit.max(config.preview.card.description_max_height))
        })
        .into();

    let image = config.preview.card.show_image.then(|| {
        container(preview_image(
            card_image,
            config.preview.card.round_image_corners,
            ContentFit::ScaleDown,
            on_hover.filter(|_| config.preview.image.animate_on_hover()),
        ))
        .padding(Padding::default().top(8))
        .height(Fit.max(config.preview.card.image_max_height))
        .into()
    });

    (title, description, image)
}

pub fn preview_content<'a, M: 'a>(
    preview: &'a Preview,
    config: &'a Config,
    theme: &'a Theme,
    on_hover: Option<fn(animated_image::hover::Request) -> M>,
) -> Element<'a, M> {
    match preview {
        Preview::Card(preview) => {
            let (title, description, image) =
                preview_card_parts(preview, config, theme, on_hover);

            container(
                column![title, description, image]
                    .spacing(8)
                    .width(Fit.max(config.preview.card.max_width)),
            )
            .padding(8)
            .into()
        }

        Preview::Image(img) => container(preview_image(
            img,
            config.preview.image.round_corners,
            ContentFit::ScaleDown,
            on_hover.filter(|_| config.preview.image.animate_on_hover()),
        ))
        .width(Fit.max(config.preview.image.max_width))
        .height(Fit.max(config.preview.image.max_height))
        .into(),
    }
}

fn preview_image<'a, M: 'a>(
    data: &'a data::Image,
    round_corners: bool,
    content_fit: ContentFit,
    on_hover: Option<fn(animated_image::hover::Request) -> M>,
) -> Element<'a, M> {
    let image = image::from_data(data, round_corners, content_fit);
    if matches!(
        data.format,
        data::image::Format::Raster(::image::ImageFormat::Gif)
    ) {
        let image = if let Some(on_hover) = on_hover {
            animated_image::on_hover(
                &data.path,
                image,
                on_hover,
                round_corners,
                content_fit,
            )
        } else {
            image
        };
        stack![
            image,
            container(
                container(
                    text("GIF")
                        .size(10)
                        .line_height(1.0)
                        .style(theme::text::secondary)
                )
                .padding(2)
                .style(theme::container::transparent_overlay)
            )
            .padding(2)
            .width(Fill)
            .height(Fill)
            .align_x(iced::alignment::Horizontal::Right)
            .align_y(iced::alignment::Vertical::Bottom)
        ]
        .into()
    } else {
        image
    }
}
