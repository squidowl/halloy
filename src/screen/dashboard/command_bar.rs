use data::{Config, buffer};
use iced::Length;
use iced::widget::{column, container, text};

use super::Focus;
use crate::widget::{Element, combo_box, double_pass, key_press};
use crate::{theme, window};

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
        buffers: &[buffer::Upstream],
        version: &data::Version,
        config: &Config,
        focus: Focus,
        resize_buffer: data::buffer::Resize,
        main_window: window::Id,
    ) -> Self {
        let state = combo_box::State::new(Command::list(
            buffers,
            config,
            focus,
            resize_buffer,
            version,
            main_window,
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
        buffers: &[buffer::Upstream],
        focus: Focus,
        resize_buffer: data::buffer::Resize,
        version: &data::Version,
        config: &'a Config,
        main_window: window::Id,
    ) -> Element<'a, Message> {
        // 1px larger than default
        let font_size =
            config.font.size.map_or(theme::TEXT_SIZE, f32::from) + 1.0;

        let combo_box =
            combo_box(&self.state, "Type a command...", None, Message::Command)
                .on_close(Message::Unfocused)
                .on_option_hovered(Message::Hovered)
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
                        Command::list(
                            buffers,
                            config,
                            focus,
                            resize_buffer,
                            version,
                            main_window,
                        )
                        .iter()
                        .map(|command| {
                            text(command.to_string()).size(font_size)
                        }),
                    )
                    .map(Element::from),
            )
            // Give it some extra width
            .padding([0, 20]),
            container(combo_box)
                .padding(1)
                .style(theme::container::tooltip)
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
    Application(Application),
    Version(Version),
    Buffer(Buffer),
    Configuration(Configuration),
    UI(Ui),
    Theme(Theme),
    Window(Window),
}

#[derive(Debug, Clone)]
pub enum Application {
    Quit,
}

#[derive(Debug, Clone)]
pub enum Version {
    Application(data::Version),
}

#[derive(Debug, Clone)]
pub enum Buffer {
    Maximize(bool),
    New,
    Close,
    Replace(buffer::Upstream),
    Popout,
    Merge,
    ToggleInternal(buffer::Internal),
}

#[derive(Debug, Clone)]
pub enum Configuration {
    Reload,
    OpenConfigDirectory,
    OpenWebsite,
    OpenCacheDirectory,
    OpenDataDirectory,
}

#[derive(Debug, Clone)]
pub enum Ui {
    ToggleSidebarVisibility,
}

#[derive(Debug, Clone)]
pub enum Window {
    ToggleFullscreen,
}

#[derive(Debug, Clone)]
pub enum Theme {
    Switch(data::Theme),
    OpenEditor,
    OpenThemesWebsite,
}

impl Command {
    pub fn list(
        buffers: &[buffer::Upstream],
        config: &Config,
        focus: Focus,
        resize_buffer: data::buffer::Resize,
        version: &data::Version,
        main_window: window::Id,
    ) -> Vec<Self> {
        let buffers = Buffer::list(buffers, focus, resize_buffer, main_window)
            .into_iter()
            .map(Command::Buffer);

        let configs = Configuration::list()
            .into_iter()
            .map(Command::Configuration);

        let uis = Ui::list().into_iter().map(Command::UI);

        let windows = Window::list().into_iter().map(Command::Window);

        let themes = Theme::list(config).into_iter().map(Command::Theme);

        let version = Version::list(version).into_iter().map(Command::Version);

        let application =
            Application::list().into_iter().map(Command::Application);

        version
            .chain(application)
            .chain(buffers)
            .chain(configs)
            .chain(themes)
            .chain(uis)
            .chain(windows)
            .collect()
    }
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Buffer(buffer) => write!(f, "Buffer: {buffer}"),
            Command::Configuration(config) => {
                write!(f, "Configuration: {config}")
            }
            Command::UI(ui) => write!(f, "UI: {ui}"),
            Command::Theme(theme) => write!(f, "Theme: {theme}"),
            Command::Version(application) => {
                write!(f, "Version: {application}")
            }
            Command::Window(window) => write!(f, "Window: {window}"),
            Command::Application(application) => {
                write!(f, "Application: {application}")
            }
        }
    }
}

impl Buffer {
    fn list(
        buffers: &[buffer::Upstream],
        focus: Focus,
        resize_buffer: data::buffer::Resize,
        main_window: window::Id,
    ) -> Vec<Self> {
        let mut list = vec![Buffer::New];
        list.extend(
            buffer::Internal::ALL
                .iter()
                .copied()
                .map(Buffer::ToggleInternal),
        );

        list.push(Buffer::Close);

        match resize_buffer {
            data::buffer::Resize::Maximize => list.push(Buffer::Maximize(true)),
            data::buffer::Resize::Restore => list.push(Buffer::Maximize(false)),
            data::buffer::Resize::None => {}
        }

        if focus.window == main_window {
            list.push(Buffer::Popout);
        } else {
            list.push(Buffer::Merge);
        }

        list.extend(buffers.iter().cloned().map(Buffer::Replace));

        list
    }
}

impl Application {
    fn list() -> Vec<Self> {
        vec![Application::Quit]
    }
}

impl Version {
    fn list(version: &data::Version) -> Vec<Self> {
        vec![Version::Application(version.clone())]
    }
}

impl Configuration {
    fn list() -> Vec<Self> {
        vec![
            Configuration::OpenConfigDirectory,
            Configuration::OpenDataDirectory,
            Configuration::OpenCacheDirectory,
            Configuration::OpenWebsite,
            Configuration::Reload,
        ]
    }
}

impl Ui {
    fn list() -> Vec<Self> {
        vec![Ui::ToggleSidebarVisibility]
    }
}

impl Window {
    fn list() -> Vec<Self> {
        vec![Window::ToggleFullscreen]
    }
}

impl Theme {
    fn list(config: &Config) -> Vec<Self> {
        Some(Self::OpenEditor)
            .into_iter()
            .chain(Some(Self::OpenThemesWebsite))
            .chain(config.appearance.all.iter().cloned().map(Self::Switch))
            .collect()
    }
}

impl std::fmt::Display for Application {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Application::Quit => write!(f, "Quit"),
        }
    }
}

impl std::fmt::Display for Window {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Window::ToggleFullscreen => write!(f, "Toggle Fullscreen"),
        }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Version::Application(version) => {
                let latest = version
                    .remote
                    .as_ref()
                    .filter(|remote| remote != &&version.current)
                    .map_or("(Latest release)".to_owned(), |remote| {
                        format!("(Latest: {remote})")
                    });

                write!(f, "{} {}", version.current, latest)
            }
        }
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
                buffer::Upstream::Server(server) => {
                    write!(f, "Change to {server}")
                }
                buffer::Upstream::Channel(server, channel) => {
                    write!(f, "Change to {channel} ({server})")
                }
                buffer::Upstream::Query(_, nick) => {
                    write!(f, "Change to {nick}")
                }
            },
            Buffer::Popout => write!(f, "Pop out buffer"),
            Buffer::Merge => write!(f, "Merge buffer"),
            Buffer::ToggleInternal(internal) => write!(f, "Toggle {internal}"),
        }
    }
}

impl std::fmt::Display for Configuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Configuration::OpenConfigDirectory => {
                write!(f, "Open config directory")
            }
            Configuration::OpenWebsite => {
                write!(f, "Open documentation website")
            }
            Configuration::Reload => write!(f, "Reload config file"),
            Configuration::OpenCacheDirectory => {
                write!(f, "Open cache directory")
            }
            Configuration::OpenDataDirectory => {
                write!(f, "Open data directory")
            }
        }
    }
}

impl std::fmt::Display for Ui {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ui::ToggleSidebarVisibility => {
                write!(f, "Toggle sidebar visibility")
            }
        }
    }
}

impl std::fmt::Display for Theme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Theme::Switch(theme) => write!(f, "Switch to {}", theme.name),
            Theme::OpenEditor => write!(f, "Open editor"),
            Theme::OpenThemesWebsite => {
                write!(f, "Discover more themes (Opens website)")
            }
        }
    }
}
