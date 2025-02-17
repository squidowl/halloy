use data::Config;
use iced::{
    alignment,
    widget::{column, container, horizontal_space, row, slider, text, Rule},
    Length,
};

use crate::{appearance::theme, widget::Element};

use super::setting_row;

#[derive(Debug, Clone)]
pub enum Message {
    Change(f64),
}

pub fn view<'a>(config: &Config) -> Element<'a, Message> {
    let scale_factor_content = {
        let content = container(column![
            slider(1.0..=3.0, config.scale_factor.into(), Message::Change),
            container(
                text(format!("{:.1}", f64::from(config.scale_factor)))
                    .style(theme::text::secondary)
                    .size(theme::TEXT_SIZE - 1.0)
            )
            .center_x(Length::Fill)
        ])
        .width(120);

        setting_row(
            "Scale Factor",
            "Application wide scale factor.",
            content,
            false,
        )
    };

    container(column![scale_factor_content]).into()
}
