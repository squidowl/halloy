use data::Config;
    use iced::{
        alignment,
        widget::{
            column, container, horizontal_space, opaque, row, slider, stack, text, vertical_space,
            Rule,
        },
        Length,
    };

    use crate::{
        appearance::theme,
        widget::{tooltip, Element},
    };

    #[derive(Debug, Clone)]
    pub enum Message {
        Change(f64),
    }

    pub fn view<'a>(config: &Config, disabled: bool) -> Element<'a, Message> {
        let content: Element<_> = {
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

            if disabled {
                let disabled = tooltip(
                    opaque(
                        container(vertical_space())
                            .style(theme::container::disabled_setting)
                            .width(Length::Fill),
                    ),
                    Some("Disabled. Configuration is defined in local config."),
                    iced::widget::tooltip::Position::Left,
                );

                stack![content, disabled].into()
            } else {
                content.into()
            }
        };

        container(column![column![
            row![
                stack![],
                column![
                    text("Scale Factor"),
                    text("Application wide scale factor.").style(theme::text::secondary),
                ]
                .max_width(200)
                .spacing(2),
                horizontal_space(),
                content
            ]
            .align_y(alignment::Vertical::Center),
            Rule::horizontal(1)
        ]
        .spacing(8)])
        .into()
    }