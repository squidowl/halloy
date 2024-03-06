use data::Config;
use iced::widget::{column, container, text};
use iced::Length;

use crate::theme;
use crate::widget::{combo_box, double_pass, key_press, Element};

#[derive(Debug, Clone)]
pub struct CommandBar {
    state: combo_box::State<Command>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Command(Command),
    Hovered(Command),
    Unfocused,
    Ignored,
}

impl CommandBar {
    pub fn new(
        buffers: &[data::Buffer],
        config: &Config,
        is_focused_buffer: bool,
        resize_buffer: data::buffer::Resize,
    ) -> Self {
        let state = combo_box::State::new(Command::list(
            buffers,
            config,
            is_focused_buffer,
            resize_buffer,
        ));
        state.focus();

        Self { state }
    }

    pub fn update(&mut self, message: Message) -> Option<Event> {
        match message {
            Message::Command(command) => Some(Event::Command(command)),
            Message::Hovered(Command::Theme(Theme::Switch(theme))) => {
                Some(Event::ThemePreview(Some(theme)))
            }
            Message::Hovered(_) => Some(Event::ThemePreview(None)),
            Message::Unfocused => Some(Event::Unfocused),
            Message::Ignored => None,
        }
    }

    pub fn view<'a>(
        &'a self,
        buffers: &[data::Buffer],
        focused_buffer: bool,
        resize_buffer: data::buffer::Resize,
        config: &'a Config,
    ) -> Element<'a, Message> {
        // 1px larger than default
        let font_size = config.font.size + 1.0;

        let combo_box = combo_box(&self.state, "Type a command...", None, Message::Command)
            .on_close(Message::Unfocused)
            .on_option_hovered(Message::Hovered)
            .style(theme::ComboBox::Default)
            .size(font_size)
            .padding([8, 8]);

        // Capture ESC so we can close the combobox manually from application
        // and prevent undesired effects
        let combo_box = key_press(
            combo_box,
            key_press::Key::Named(key_press::Named::Escape),
            key_press::Modifiers::default(),
            Message::Ignored,
        );

        double_pass(
            // Layout should be based on the Shrink text size width of largest option
            column(
                std::iter::once(text("Type a command...").size(font_size))
                    .chain(
                        Command::list(buffers, config, focused_buffer, resize_buffer)
                            .iter()
                            .map(|command| text(command).size(font_size)),
                    )
                    .map(Element::from),
            )
            // Give it some extra width
            .padding([0, 20]),
            container(combo_box)
                .padding(1)
                .style(theme::Container::Context)
                .width(Length::Fill),
        )
    }
}

pub enum Event {
    Command(Command),
    ThemePreview(Option<data::Theme>),
    Unfocused,
}

#[derive(Debug, Clone)]
pub enum Command {
    Buffer(Buffer),
    Configuration(Configuration),
    UI(Ui),
    Theme(Theme),
}

#[derive(Debug, Clone)]
pub enum Buffer {
    Maximize(bool),
    New,
    Close,
    Replace(data::Buffer),
}

#[derive(Debug, Clone)]
pub enum Configuration {
    Open,
}

#[derive(Debug, Clone)]
pub enum Ui {
    ToggleSidebarVisibility,
}

#[derive(Debug, Clone)]
pub enum Theme {
    Switch(data::Theme),
}

impl Command {
    pub fn list(
        buffers: &[data::Buffer],
        config: &Config,
        is_focused_buffer: bool,
        resize_buffer: data::buffer::Resize,
    ) -> Vec<Self> {
        let buffers = Buffer::list(buffers, is_focused_buffer, resize_buffer)
            .into_iter()
            .map(Command::Buffer);

        let configs = Configuration::list()
            .into_iter()
            .map(Command::Configuration);

        let uis = Ui::list().into_iter().map(Command::UI);

        let themes = Theme::list(config).into_iter().map(Command::Theme);

        buffers.chain(configs).chain(themes).chain(uis).collect()
    }
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Buffer(buffer) => write!(f, "Buffer: {}", buffer),
            Command::Configuration(config) => write!(f, "Configuration: {}", config),
            Command::UI(ui) => write!(f, "UI: {}", ui),
            Command::Theme(theme) => write!(f, "Theme: {}", theme),
        }
    }
}

impl Buffer {
    fn list(
        buffers: &[data::Buffer],
        is_focused_buffer: bool,
        resize_buffer: data::buffer::Resize,
    ) -> Vec<Self> {
        let mut list = vec![Buffer::New];

        if is_focused_buffer {
            list.push(Buffer::Close);

            match resize_buffer {
                data::buffer::Resize::Maximize => list.push(Buffer::Maximize(true)),
                data::buffer::Resize::Restore => list.push(Buffer::Maximize(false)),
                data::buffer::Resize::None => {}
            }

            list.extend(buffers.iter().cloned().map(Buffer::Replace));
        }

        list
    }
}

impl Configuration {
    fn list() -> Vec<Self> {
        vec![Configuration::Open]
    }
}

impl Ui {
    fn list() -> Vec<Self> {
        vec![Ui::ToggleSidebarVisibility]
    }
}

impl Theme {
    fn list(config: &Config) -> Vec<Self> {
        config
            .themes
            .all
            .iter()
            .cloned()
            .map(Self::Switch)
            .collect()
    }
}

impl std::fmt::Display for Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Buffer::Maximize(maximize) => {
                write!(
                    f,
                    "{}",
                    if *maximize {
                        "Maximize"
                    } else {
                        "Restore size"
                    }
                )
            }
            Buffer::New => write!(f, "New buffer"),
            Buffer::Close => write!(f, "Close buffer"),
            Buffer::Replace(buffer) => match buffer {
                data::Buffer::Server(server) => write!(f, "Change to {}", server),
                data::Buffer::Channel(server, channel) => {
                    write!(f, "Change to {} ({})", channel, server)
                }
                data::Buffer::Query(_, nick) => write!(f, "Change to {}", nick),
            },
        }
    }
}

impl std::fmt::Display for Configuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Configuration::Open => write!(f, "Open directory"),
        }
    }
}

impl std::fmt::Display for Ui {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ui::ToggleSidebarVisibility => write!(f, "Toggle sidebar visibility"),
        }
    }
}

impl std::fmt::Display for Theme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Theme::Switch(theme) => write!(f, "Switch to {}", theme.name),
        }
    }
}
