use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

use chrono::{DateTime, Local, NaiveDate, Utc};
use data::buffer::RightAlignmentWidths;
use data::command::Irc;
use data::config::actions::{ChannelClickAction, NicknameClickAction};
use data::config::buffer::{CondensationIcon, HideConsecutiveEnabled};
use data::dashboard::BufferAction;
use data::isupport::ChatHistoryState;
use data::message::{self, Limit};
use data::preview::{self, Previews};
use data::rate_limit::TokenPriority;
use data::reaction::Reaction;
use data::server::Server;
use data::target::{self, Target};
use data::{
    Config, Image, Preview, User, client, history, isupport, metadata, reaction,
};
use iced::border::Radius;
use iced::widget::{
    self, Scrollable, button, column, container, row, rule, scrollable, space,
    text,
};
use iced::{Length, Size, Task, padding};
use tokio::time;

use self::correct_viewport::correct_viewport;
use self::keyed::keyed;
use super::{context_menu, input_view};
use crate::widget::user_display::UserDisplay;
use crate::widget::{
    Element, double_pass, key_press, notify_visibility, on_key, on_resize,
};
use crate::{Theme, buffer, font, theme};

const SCROLL_TO_TIMEOUT: Duration = Duration::from_millis(200);
/// Pages of off-screen messages to keep rendered above and below the viewport
const BUFFER_PAGES: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollAnchor {
    #[default]
    Top,
    Bottom,
}

const HIGHLIGHT_HOLD_MS: u64 = 2000;
const HIGHLIGHT_ALPHA_START: f32 = 1.0;
const HOVER_HIGHLIGHT_ALPHA: f32 = 0.4;
const HIGHLIGHT_ALPHA_TICK_MS: u64 = 20;
const HIGHLIGHT_ALPHA_STEP: f32 =
    HIGHLIGHT_ALPHA_START / (400.0 / HIGHLIGHT_ALPHA_TICK_MS as f32);

#[derive(Debug, Clone)]
pub enum Message {
    Scrolled {
        count: usize,
        has_more_older_messages: bool,
        has_more_newer_messages: bool,
        oldest: DateTime<Utc>,
        status: Status,
        viewport: scrollable::Viewport,
    },
    ContextMenu(context_menu::Message),
    Link(message::Link),
    ImagePreview(Image),
    ScrollTo(keyed::Hit),
    RequestOlderChatHistory,
    EnteringViewport(message::Hash, Vec<url::Url>),
    ExitingViewport(message::Hash),
    ReplyPreviewHovered(message::Hash, message::Hash, Vec<url::Url>),
    ReplyPreviewUnhovered(message::Hash),
    EnteredViewport(message::Hash),
    ExitedViewport(message::Hash),
    PreviewHovered(message::Hash, usize),
    PreviewUnhovered(message::Hash, usize),
    HidePreview(message::Hash, url::Url),
    MarkAsRead,
    ContentResized(Size),
    PendingScrollTo,
    FadeHighlight(message::Hash, u64),
    HeightsCollected(Vec<(keyed::Key, f32)>),
    Reacted {
        msgid: message::Id,
        text: Cow<'static, str>,
    },
    Unreacted {
        msgid: message::Id,
        text: Cow<'static, str>,
    },
    NavigateFocus(Direction),
    OpenFocusMenu,
    OpenNickFocusMenu,
    FocusMenuMove(Direction),
    FocusMenuActivate(usize),
    FocusMenuClose,
    FocusMenuDismiss,
}

impl From<context_menu::Message> for Message {
    fn from(message: context_menu::Message) -> Self {
        Message::ContextMenu(message)
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    ContextMenu(context_menu::Event),
    OpenBuffer(Server, Target, BufferAction),
    GoToMessage(Server, target::Channel, message::Hash, BufferAction),
    RequestOlderChatHistory,
    PreviewChanged,
    HidePreview(history::Kind, message::Hash, url::Url),
    MarkAsRead,
    OpenUrl(String),
    ImagePreview(Image),
    ExpandMessage(DateTime<Utc>, message::Hash),
    ContractMessage(DateTime<Utc>, message::Hash),
    ExitFocus,
    FocusAction(input_view::FocusAction),
    FocusContextAction(context_menu::Message),
}

/// A keyboard-navigable menu of focus actions, anchored to a focused message.
/// Opened to the right of the message it shows message actions; opened to the
/// left it shows the message author's (nick) actions.
#[derive(Debug, Clone)]
pub struct FocusMenu {
    hash: message::Hash,
    selection: usize,
    content: FocusMenuContent,
}

#[derive(Debug, Clone)]
enum FocusMenuContent {
    Message(Vec<FocusEntry>),
    Nick(NickFocusData),
}

/// Owned data needed to render and activate the nick (user) actions menu. The
/// entries mirror the right-click user menu; references for rendering are
/// rebuilt from these owned fields at view time.
#[derive(Debug, Clone)]
struct NickFocusData {
    server: Server,
    channel: Option<target::Channel>,
    prefix: Vec<isupport::PrefixMap>,
    user: User,
    current_user: Option<User>,
    entries: Vec<context_menu::Entry>,
}

#[derive(Debug, Clone)]
struct FocusEntry {
    label: String,
    separator_before: bool,
    action: FocusEntryAction,
}

#[derive(Debug, Clone)]
enum FocusEntryAction {
    Message(input_view::FocusAction),
    Context(context_menu::Message),
    // Activates the same behavior as clicking the link (e.g. opening a channel).
    Link(message::Link),
}

/// Non-actionable nick menu rows (the avatar / metadata header and separators)
/// that keyboard navigation skips over.
fn nick_entry_actionable(entry: context_menu::Entry) -> bool {
    !matches!(
        entry,
        context_menu::Entry::UserInfo
            | context_menu::Entry::UserMetadata
            | context_menu::Entry::HorizontalRule
    )
}

impl FocusMenu {
    fn entry_count(&self) -> usize {
        match &self.content {
            FocusMenuContent::Message(entries) => entries.len(),
            FocusMenuContent::Nick(data) => data.entries.len(),
        }
    }

    fn is_actionable(&self, index: usize) -> bool {
        match &self.content {
            FocusMenuContent::Message(_) => true,
            FocusMenuContent::Nick(data) => data
                .entries
                .get(index)
                .copied()
                .is_some_and(nick_entry_actionable),
        }
    }

    fn first_actionable(&self) -> usize {
        (0..self.entry_count())
            .find(|index| self.is_actionable(*index))
            .unwrap_or(0)
    }

    fn move_selection(&mut self, direction: Direction) {
        let len = self.entry_count();
        if len == 0 {
            return;
        }

        let mut next = self.selection;
        for _ in 0..len {
            next = match direction {
                Direction::Up => (next + len - 1) % len,
                Direction::Down => (next + 1) % len,
            };

            if self.is_actionable(next) {
                self.selection = next;
                return;
            }
        }
    }

    /// The message this menu is anchored to.
    pub fn hash(&self) -> message::Hash {
        self.hash
    }

    /// Whether this is the nick (user) actions menu, anchored to the nick,
    /// rather than the message actions menu, anchored to the content.
    pub fn is_nick(&self) -> bool {
        matches!(self.content, FocusMenuContent::Nick(_))
    }
}

/// Renders the open focus action menu. Anchored by the caller to the menu's
/// target (the nick or the message content) within the message layout.
pub fn focus_menu_overlay<'a>(
    menu: &'a FocusMenu,
    registry: &'a dyn metadata::Registry,
    previews: Option<&'a preview::Collection>,
    theme: &'a Theme,
    config: &'a Config,
) -> Element<'a, Message> {
    match &menu.content {
        FocusMenuContent::Message(entries) => {
            focus_menu_view(entries, menu.selection, theme, config)
        }
        FocusMenuContent::Nick(data) => nick_focus_menu_view(
            data,
            menu.selection,
            registry,
            previews,
            theme,
            config,
        ),
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Kind<'a> {
    Server(&'a Server),
    Channel(&'a Server, &'a target::Channel),
    Query(&'a Server, &'a target::Query),
    Logs,
    Highlights,
}

impl Kind<'_> {
    pub(crate) fn server(&self) -> Option<&Server> {
        match self {
            Kind::Server(server)
            | Kind::Channel(server, _)
            | Kind::Query(server, _) => Some(server),
            Kind::Logs | Kind::Highlights => None,
        }
    }
}

impl From<Kind<'_>> for history::Kind {
    fn from(value: Kind<'_>) -> Self {
        match value {
            Kind::Server(server) => history::Kind::Server(server.clone()),
            Kind::Channel(server, channel) => {
                history::Kind::Channel(server.clone(), channel.clone())
            }
            Kind::Query(server, nick) => {
                history::Kind::Query(server.clone(), nick.clone())
            }
            Kind::Logs => history::Kind::Logs,
            Kind::Highlights => history::Kind::Highlights,
        }
    }
}

pub trait LayoutMessage<'a> {
    fn format(
        &self,
        message: &'a data::Message,
        right_alignment_widths: Option<RightAlignmentWidths>,
        hide_timestamp: bool,
        hide_nickname: bool,
        visible_for_source: Option<
            &impl Fn(&Preview, &message::Source) -> bool,
        >,
        visible_url_messages: &HashMap<message::Hash, Vec<url::Url>>,
        hovered_preview: Option<(message::Hash, usize)>,
        hovered_reply: Option<message::Hash>,
        focused_link: Option<usize>,
    ) -> Option<Element<'a, Message>>;
}

impl<'a, T> LayoutMessage<'a> for T
where
    T: Fn(
        &'a data::Message,
        Option<RightAlignmentWidths>,
        bool,
        bool,
    ) -> Option<Element<'a, Message>>,
{
    fn format(
        &self,
        message: &'a data::Message,
        right_alignment_widths: Option<RightAlignmentWidths>,
        hide_timestamp: bool,
        hide_nickname: bool,
        _visible_for_source: Option<
            &impl Fn(&Preview, &message::Source) -> bool,
        >,
        _visible_url_messages: &HashMap<message::Hash, Vec<url::Url>>,
        _hovered_preview: Option<(message::Hash, usize)>,
        _hovered_reply: Option<message::Hash>,
        _focused_link: Option<usize>,
    ) -> Option<Element<'a, Message>> {
        self(
            message,
            right_alignment_widths,
            hide_timestamp,
            hide_nickname,
        )
    }
}

/// Check if a message has a visible image preview
fn has_visible_preview(
    message: &data::Message,
    state: &State,
    previews: Option<Previews>,
    visible_for_source: &Option<impl Fn(&Preview, &message::Source) -> bool>,
) -> bool {
    if let message::Content::Fragments(fragments) = &message.content
        && let Some(previews) = previews
        && let Some(visible_urls) =
            state.visible_url_messages.get(&message.hash)
    {
        return fragments.iter().filter_map(message::Fragment::url).any(
            |url| {
                // Check if URL is hidden by user
                if message.hidden_urls.contains(url) {
                    return false;
                }

                // Check if URL is in visible URLs list
                if !visible_urls.contains(url) {
                    return false;
                }

                // Check if preview is loaded and visible for source
                if let Some(preview::State::Loaded(preview)) = previews.get(url)
                {
                    let is_visible_for_source =
                        if let Some(visible_for_source) = visible_for_source {
                            visible_for_source(preview, message.target.source())
                        } else {
                            true
                        };

                    return is_visible_for_source;
                }

                false
            },
        );
    }
    false
}

fn is_consecutive_user_message(
    message: &data::Message,
    prev_message: Option<&data::Message>,
    duration: Option<chrono::TimeDelta>,
    config: &Config,
) -> bool {
    matches!(message.target.source(), message::Source::User(_))
        && prev_message.is_some_and(|prev_message| {
            if duration.is_none_or(|duration| {
                message.server_time - prev_message.server_time < duration
            }) && let message::Source::User(user) = message.target.source()
                && let message::Source::User(prev_user) =
                    prev_message.target.source()
            {
                user.has_matching_display(
                    prev_user,
                    config.buffer.nickname.show_access_levels,
                    config.buffer.nickname.show_bot_icon,
                )
            } else {
                false
            }
        })
}

fn menu_separator<'a>(width: Length) -> Element<'a, Message> {
    match width {
        Length::Fill => container(rule::horizontal(1)).padding([0, 6]).into(),
        _ => widget::Space::new().width(width).height(1).into(),
    }
}

fn build_focus_entries(
    message: &data::Message,
    focused_link: Option<usize>,
    server: &Server,
    clients: &client::Map,
    previews: Option<&Previews>,
    config: &Config,
) -> Vec<FocusEntry> {
    let focus_target = focused_link
        .and_then(|index| message_focus_target_at(message, index))
        .or_else(|| message_single_url(message).map(FocusTarget::Url));

    let message_entry =
        |action: input_view::FocusAction, separator_before| FocusEntry {
            label: context_menu::Entry::from(action).label().to_string(),
            separator_before,
            action: FocusEntryAction::Message(action),
        };

    let has_id = message.id.is_some();
    let mut interaction_actions = vec![];
    if focused_link.is_none() {
        if has_id && clients.get_server_can_send_replies(server) {
            interaction_actions.push(input_view::FocusAction::Reply);
        }
        if has_id && clients.get_server_can_send_reactions(server) {
            interaction_actions
                .push(input_view::FocusAction::OpenReactionModal);
        }
        if has_id && clients.get_server_can_redact(server) {
            interaction_actions.push(input_view::FocusAction::Redact);
        }
    }
    let interaction_entries = interaction_actions
        .into_iter()
        .enumerate()
        .map(|(index, action)| message_entry(action, index == 0));

    match focus_target {
        Some(FocusTarget::Url(url)) => {
            let url_string = url.to_string();

            let mut entries = vec![
                FocusEntry {
                    label: "Copy URL".to_string(),
                    separator_before: false,
                    action: FocusEntryAction::Context(
                        context_menu::Message::CopyUrl(url_string.clone()),
                    ),
                },
                FocusEntry {
                    label: "Open URL".to_string(),
                    separator_before: false,
                    action: FocusEntryAction::Context(
                        context_menu::Message::OpenUrl(url_string.clone()),
                    ),
                },
            ];

            if let Some(is_hidden) = previews.and_then(|previews| {
                previews.is_hidden_for_url(message, &url, &config.preview)
            }) {
                let (toggle_label, toggle_message) = if is_hidden {
                    (
                        "Show Preview",
                        context_menu::Message::ShowPreview(
                            message.hash,
                            url_string,
                        ),
                    )
                } else {
                    (
                        "Hide Preview",
                        context_menu::Message::HidePreview(
                            message.hash,
                            url_string,
                        ),
                    )
                };

                entries.push(FocusEntry {
                    label: toggle_label.to_string(),
                    separator_before: true,
                    action: FocusEntryAction::Context(toggle_message),
                });
            }

            entries.extend(interaction_entries);

            entries
        }
        Some(FocusTarget::Channel(channel)) => {
            let target = target::Channel::from_str(
                &channel,
                clients.get_server_chantypes_or_default(server),
                clients.get_server_casemapping_or_default(server),
            );

            let buffer_action = match config.actions.buffer.click_channel_name {
                ChannelClickAction::OpenChannel(buffer_action) => buffer_action,
                ChannelClickAction::Noop => BufferAction::default(),
            };

            let mut entries = vec![FocusEntry {
                label: "Open channel".to_string(),
                separator_before: false,
                action: FocusEntryAction::Link(message::Link::Channel(
                    server.clone(),
                    target,
                    buffer_action,
                )),
            }];

            entries.extend(interaction_entries);

            entries
        }
        None => {
            // Parent message is focused - offer message actions
            std::iter::once(message_entry(
                input_view::FocusAction::CopyText,
                false,
            ))
            .chain(interaction_entries)
            .collect()
        }
    }
}

fn focus_menu_view<'a>(
    entries: &[FocusEntry],
    selection: usize,
    theme: &Theme,
    config: &Config,
) -> Element<'a, Message> {
    let build = |width: Length| -> Element<'a, Message> {
        let entries = entries.iter().enumerate().fold(
            column![],
            |col, (index, entry)| {
                let col = if entry.separator_before {
                    col.push(menu_separator(width))
                } else {
                    col
                };

                let selected = index == selection;

                col.push(context_menu::menu_button(
                    entry.label.clone(),
                    Some(Message::FocusMenuActivate(index)),
                    selected,
                    width,
                    theme,
                    config,
                ))
            },
        );

        container(entries)
            .padding(4)
            .style(theme::container::tooltip)
            .into()
    };

    let panel = double_pass(build(Length::Shrink), build(Length::Fill));

    on_key(panel, move |key, modifiers| {
        use key_press::{Key, Named};

        match key {
            Key::Named(Named::ArrowUp) => {
                Some(Message::FocusMenuMove(Direction::Up))
            }
            Key::Named(Named::ArrowDown) => {
                Some(Message::FocusMenuMove(Direction::Down))
            }
            Key::Named(Named::Tab) => {
                Some(Message::FocusMenuMove(if modifiers.shift() {
                    Direction::Up
                } else {
                    Direction::Down
                }))
            }
            Key::Named(Named::ArrowLeft) => Some(Message::FocusMenuClose),
            Key::Named(Named::ArrowRight | Named::Enter) => {
                Some(Message::FocusMenuActivate(selection))
            }
            _ => None,
        }
    })
}

/// Renders the nick (user) actions menu — the full right-click user menu
/// (avatar / metadata header plus actions) made keyboard-navigable. The
/// opposite focus key (right) returns to the message body.
fn nick_focus_menu_view<'a>(
    data: &'a NickFocusData,
    selection: usize,
    registry: &'a dyn metadata::Registry,
    previews: Option<&'a preview::Collection>,
    theme: &'a Theme,
    config: &'a Config,
) -> Element<'a, Message> {
    let avatar = previews.and_then(|previews| {
        context_menu::user_avatar(&data.user, registry, previews)
    });

    let build = |width: Length| -> Element<'a, Message> {
        let entries = data.entries.iter().enumerate().fold(
            column![],
            |col, (index, entry)| {
                let context = context_menu::Context::User {
                    server: &data.server,
                    prefix: &data.prefix,
                    channel: data.channel.as_ref(),
                    registry,
                    avatar: avatar.clone(),
                    user: &data.user,
                    current_user: data.current_user.as_ref(),
                };

                // Reuse the right-click row rendering (incl. selection
                // highlight); route any click through the same activation path
                // as keyboard Enter so focus mode is exited consistently.
                let element = (*entry)
                    .view(
                        Some(context),
                        width,
                        config,
                        theme,
                        index == selection,
                    )
                    .map(move |_| Message::FocusMenuActivate(index));

                col.push(element)
            },
        );

        container(entries)
            .padding(4)
            .style(theme::container::tooltip)
            .into()
    };

    let panel = double_pass(build(Length::Shrink), build(Length::Fill));

    on_key(panel, move |key, _modifiers| {
        use key_press::{Key, Named};

        match key {
            Key::Named(Named::ArrowUp) => {
                Some(Message::FocusMenuMove(Direction::Up))
            }
            Key::Named(Named::ArrowDown) => {
                Some(Message::FocusMenuMove(Direction::Down))
            }
            Key::Named(Named::Tab) => {
                Some(Message::FocusMenuMove(Direction::Down))
            }
            // The opposite focus key refocuses the message body.
            Key::Named(Named::ArrowRight) => Some(Message::FocusMenuClose),
            // Left (the direction this menu opened toward) and Enter activate
            // the selected item, mirroring Right in the message actions menu.
            Key::Named(Named::ArrowLeft | Named::Enter) => {
                Some(Message::FocusMenuActivate(selection))
            }
            _ => None,
        }
    })
}

pub(crate) fn focus_outline<'a>(
    inner: Element<'a, Message>,
) -> Element<'a, Message> {
    use iced::advanced::{Layout, Renderer as _, mouse, renderer, widget};

    crate::widget::decorate(inner)
        .draw(
            move |_state: &(),
                  inner: &Element<'a, Message>,
                  tree: &widget::Tree,
                  renderer: &mut crate::widget::Renderer,
                  theme: &Theme,
                  style: &renderer::Style,
                  layout: Layout<'_>,
                  cursor: mouse::Cursor,
                  viewport: &iced::Rectangle| {
                inner.as_widget().draw(
                    tree, renderer, theme, style, layout, cursor, viewport,
                );

                let buffer = theme.styles().buffer;
                let color = buffer.focus.unwrap_or(buffer.border_selected);

                let b = layout.bounds();
                let bounds = iced::Rectangle {
                    x: b.x - 2.0,
                    y: b.y - 2.0,
                    width: b.width + 2.0,
                    height: b.height + 2.0,
                };

                renderer.fill_quad(
                    renderer::Quad {
                        bounds,
                        border: iced::Border {
                            width: 2.0,
                            color,
                            radius: Radius::new(3.0),
                        },
                        ..renderer::Quad::default()
                    },
                    iced::Color::TRANSPARENT,
                );
            },
        )
        .into()
}

pub fn view<'a>(
    state: &State,
    focused_message: Option<message::Hash>,
    kind: Kind,
    history: &'a history::Manager,
    previews: Option<Previews<'a>>,
    visible_for_source: Option<impl Fn(&Preview, &message::Source) -> bool>,
    chathistory_state: Option<ChatHistoryState>,
    reserved_bottom_padding: f32,
    config: &'a Config,
    theme: &'a Theme,
    formatter: impl LayoutMessage<'a> + 'a,
    registry: &'a dyn metadata::Registry,
) -> Element<'a, Message> {
    let divider_font_size =
        config.font.size.map_or(theme::TEXT_SIZE, f32::from) - 1.0;

    let Some(history::View {
        has_more_older_messages,
        has_more_newer_messages,
        old_messages,
        new_messages,
        cleared,
        ..
    }) = history.get_messages(&kind.into(), Some(state.limit), config)
    else {
        return column![].into();
    };

    let top_row = if !cleared
        && let (false, Some(chathistory_state)) =
            (has_more_older_messages, chathistory_state)
    {
        let (content, message) = match chathistory_state {
            ChatHistoryState::Exhausted => {
                ("No Older Chat History Messages Available", None)
            }
            ChatHistoryState::PendingRequest => ("...", None),
            ChatHistoryState::Ready => (
                "Request Older Chat History Messages",
                Some(Message::RequestOlderChatHistory),
            ),
        };

        let top_row_button = button(text(content).size(divider_font_size))
            .padding([3, 5])
            .style(|theme, status| {
                theme::button::secondary(theme, status, false)
            })
            .on_press_maybe(message);

        Some(
            row![space::horizontal(), top_row_button, space::horizontal()]
                .padding(padding::top(4).bottom(6))
                .width(Length::Fill)
                .align_y(iced::Alignment::Center),
        )
    } else {
        None
    };

    let count = old_messages.len() + new_messages.len();
    let oldest = old_messages
        .iter()
        .chain(&new_messages)
        .next()
        .map_or_else(Utc::now, |message| message.server_time);
    let status = state.status;

    let right_alignment_widths =
        config.buffer.nickname.alignment.is_right().then_some({
            let max_prefixes_width = old_messages
                .iter()
                .chain(&new_messages)
                .filter_map(|message| prefixes_width(message, config))
                .fold(0.0, f32::max);

            let max_timestamp_width = old_messages
                .iter()
                .chain(&new_messages)
                .filter_map(|message| timestamp_width(message, config))
                .fold(0.0, f32::max);

            let max_nick_width = old_messages
                .iter()
                .chain(&new_messages)
                .filter_map(|message| match message.target.source() {
                    message::Source::User(user) => {
                        let user_display = UserDisplay::new(
                            user,
                            config.buffer.nickname.show_access_levels,
                            config.buffer.nickname.show_bot_icon,
                            registry,
                            &config.display.nickname,
                            config.buffer.nickname.truncate,
                            config.display.truncation_character,
                            Some(&config.buffer.nickname.brackets),
                            true,
                        );

                        Some(user_display.width(config) + 1.0)
                    }
                    _ => None,
                })
                .fold(0.0, f32::max);

            let range_end_timestamp_width =
                if config.buffer.server_messages.condense.any() {
                    old_messages
                        .iter()
                        .chain(&new_messages)
                        .filter_map(|message| {
                            if let message::Source::Internal(
                                message::source::Internal::Condensed(
                                    end_server_time,
                                ),
                            ) = message.target.source()
                                && message.server_time != *end_server_time
                            {
                                config
                                    .buffer
                                    .format_range_end_timestamp(end_server_time)
                                    .map(|(dash, end_timestamp)| {
                                        let condensation_icon = !matches!(
                                            config
                                                .buffer
                                                .server_messages
                                                .condense
                                                .icon,
                                            CondensationIcon::None
                                        );

                                        let range_end_timestamp_width =
                                            font::width_from_str(
                                                &(if condensation_icon {
                                                    format!(
                                                        "{dash}{end_timestamp} "
                                                    )
                                                } else {
                                                    format!(
                                                        "{dash}{end_timestamp}"
                                                    )
                                                }),
                                                &config.font,
                                            );

                                        range_end_timestamp_width
                                            + if condensation_icon {
                                                font::width_of_message_marker(
                                                    &config.font,
                                                )
                                            } else {
                                                0.0
                                            }
                                            + 1.0
                                    })
                            } else {
                                None
                            }
                        })
                        .fold(0.0, f32::max)
                } else {
                    0.0
                };

            let message_marker_width =
                font::width_of_message_marker(&config.font) + 1.0;

            let max_middle_width = max_nick_width
                .max(range_end_timestamp_width)
                .max(message_marker_width);

            RightAlignmentWidths {
                prefixes: max_prefixes_width,
                timestamp: max_timestamp_width,
                middle: max_middle_width,
            }
        });

    let message_rows = |last_date: Option<NaiveDate>,
                        messages: &[&'a data::Message]| {
        messages
            .iter()
            .scan(Option::<&data::Message>::None, |prev_message, message| {
                let hide_timestamp =
                    if let HideConsecutiveEnabled::Enabled(duration) =
                        config.buffer.timestamp.hide_consecutive.enabled
                    {
                        message.reply_to.is_none()
                            && is_consecutive_user_message(
                                message,
                                *prev_message,
                                duration,
                                config,
                            )
                    } else {
                        false
                    };

                let hide_nickname =
                    if let HideConsecutiveEnabled::Enabled(duration) =
                        config.buffer.nickname.hide_consecutive.enabled
                    {
                        !config.buffer.nickname.alignment.is_top()
                        && message.reply_to.is_none()
                        && is_consecutive_user_message(
                            message,
                            *prev_message,
                            duration,
                            config
                        )
                        // don't hide if prev message has visible preview (when show_after_previews is enabled)
                        && !(config
                            .buffer
                            .nickname
                            .hide_consecutive
                            .show_after_previews
                            && prev_message.is_some_and(|prev_msg| {
                                has_visible_preview(
                                    prev_msg,
                                    state,
                                    previews,
                                    &visible_for_source,
                                )
                            }))
                    } else {
                        false
                    };

                *prev_message = Some(message);

                Some(
                    formatter
                        .format(
                            message,
                            right_alignment_widths,
                            hide_timestamp,
                            hide_nickname,
                            visible_for_source.as_ref(),
                            &state.visible_url_messages,
                            state.hovered_preview,
                            state.hover_highlighted_message,
                            if focused_message == Some(message.hash)
                                && !state.focus_menu.as_ref().is_some_and(
                                    |menu| menu.hash == message.hash,
                                )
                            {
                                state.focused_link
                            } else {
                                None
                            },
                        )
                        .map(|element| (message, element)),
                )
            })
            .flatten()
            .scan(last_date, |last_date, (message, element)| {
                let date =
                    message.server_time.with_timezone(&Local).date_naive();

                let is_new_day = last_date.is_none_or(|prev| date > prev);

                *last_date = Some(date);

                let element = if focused_message == Some(message.hash)
                    && state.focused_link.is_none()
                    && !state
                        .focus_menu
                        .as_ref()
                        .is_some_and(|menu| menu.hash == message.hash)
                {
                    // Only show focus on the whole message when no link/preview
                    focus_outline(container(element).width(Length::Fill).into())
                } else if let Some((hash, alpha)) = state.highlighted_message
                    && hash == message.hash
                {
                    container(element)
                        .width(Length::Fill)
                        .style(move |theme| {
                            theme::container::highlighted_message(theme, alpha)
                        })
                        .into()
                } else if state.hover_highlighted_message == Some(message.hash)
                {
                    container(element)
                        .width(Length::Fill)
                        .style(move |theme| {
                            theme::container::highlighted_message(
                                theme,
                                HOVER_HIGHLIGHT_ALPHA,
                            )
                        })
                        .into()
                } else {
                    element
                };

                let element = {
                    let is_visible =
                        state.visible_messages.contains(&message.hash);
                    if is_visible {
                        notify_visibility(
                            element,
                            0.0,
                            notify_visibility::When::MostlyOutside,
                            message.hash,
                            Message::ExitedViewport(message.hash),
                        )
                    } else {
                        notify_visibility(
                            element,
                            0.0,
                            notify_visibility::When::MostlyContained,
                            message.hash,
                            Message::EnteredViewport(message.hash),
                        )
                    }
                };

                let content = if is_new_day
                    && config.buffer.date_separators.show
                {
                    column![
                        row![
                            container(
                                rule::horizontal(1).style(theme::rule::date)
                            )
                            .width(Length::Fill)
                            .padding(padding::right(6)),
                            text(config.buffer.format_date_separator(&date))
                                .size(divider_font_size)
                                .style(theme::text::date_separator)
                                .font_maybe(
                                    theme::font_style::secondary(theme)
                                        .map(font::get)
                                ),
                            container(
                                rule::horizontal(1).style(theme::rule::date)
                            )
                            .width(Length::Fill)
                            .padding(padding::left(6))
                        ]
                        .padding(2)
                        .align_y(iced::Alignment::Center),
                        element
                    ]
                    .into()
                } else {
                    element
                };

                Some(keyed(keyed::Key::message(message), content))
            })
            .collect::<Vec<_>>()
    };

    let line_spacing = config.buffer.line_spacing;

    // Only create widgets for messages near the viewport, use height
    // spacers for the rest so we doesn't lay out thousands of children
    let row_height =
        theme::resolve_line_height(&config.font) + line_spacing as f32;
    let total = old_messages.len() + new_messages.len();
    let visible = (state.pane_size.height / row_height).ceil() as usize;
    let buffer = visible * BUFFER_PAGES;
    let render_budget = visible + 2 * buffer;

    let msg_height = |m: &&data::Message| -> f32 {
        state
            .height_cache
            .get(&keyed::Key::Message(m.hash))
            .copied()
            .map_or(row_height, |h| h + line_spacing as f32)
    };
    let div_height = state
        .height_cache
        .get(&keyed::Key::Divider)
        .copied()
        .unwrap_or_default();

    let (render_start, render_end) = if state.pending_scroll_to.is_some()
        || state.is_scrolling_to
        || total <= render_budget
    {
        (0, total)
    } else {
        let first_visible = match state.status {
            Status::Bottom => {
                let offset = state.last_scroll_offset;
                let mut acc = 0.0_f32;
                let mut from_bottom = 0;
                for m in old_messages.iter().chain(&new_messages).rev() {
                    if from_bottom == new_messages.len() {
                        acc += div_height;
                        if acc > offset {
                            break;
                        }
                    }

                    acc += msg_height(m);
                    if acc > offset {
                        break;
                    }
                    from_bottom += 1;
                }
                total.saturating_sub(from_bottom + visible)
            }
            Status::Unlocked => {
                let offset = state.last_scroll_offset;
                let mut acc = 0.0_f32;
                let mut idx = 0;
                for m in old_messages.iter().chain(&new_messages) {
                    if idx == old_messages.len() {
                        acc += div_height;
                        if acc > offset {
                            break;
                        }
                    }

                    acc += msg_height(m);
                    if acc > offset {
                        break;
                    }
                    idx += 1;
                }
                idx
            }
        };

        (
            first_visible.saturating_sub(buffer),
            (first_visible + visible + buffer).min(total),
        )
    };

    let old_start = render_start.min(old_messages.len());
    let old_end = render_end.min(old_messages.len());
    let new_start = render_start
        .saturating_sub(old_messages.len())
        .min(new_messages.len());
    let new_end = render_end
        .saturating_sub(old_messages.len())
        .min(new_messages.len());

    let date_of =
        |m: &data::Message| m.server_time.with_timezone(&Local).date_naive();

    let old_last_date = old_start
        .checked_sub(1)
        .and_then(|i| old_messages.get(i))
        .map(|m| date_of(m));

    let new_last_date = new_start
        .checked_sub(1)
        .and_then(|i| new_messages.get(i))
        .map(|m| date_of(m))
        .or_else(|| old_messages.last().map(|m| date_of(m)));

    let old = message_rows(old_last_date, &old_messages[old_start..old_end]);
    let new = message_rows(new_last_date, &new_messages[new_start..new_end]);

    let top_spacer = (render_start > 0).then(|| {
        let h: f32 = old_messages[..old_start]
            .iter()
            .chain(&new_messages[..new_start])
            .map(&msg_height)
            .sum();
        space::vertical().height(h)
    });
    let bottom_spacer = (render_end < total).then(|| {
        let h: f32 = old_messages[old_end..]
            .iter()
            .chain(&new_messages[new_end..])
            .map(&msg_height)
            .sum();
        space::vertical().height(h)
    });

    let show_backlog_divider = if old.is_empty() {
        // If all newer messages in viewport, only show backlog divider at the top
        // if we don't have any older messages at all (we're scrolled all the way up)
        !has_more_older_messages
    } else {
        // Always show backlog divider after any visible older messages
        if config.buffer.backlog_separator.hide_when_all_read {
            !new_messages.is_empty()
        } else {
            true
        }
    };

    let divider = if show_backlog_divider {
        match &config.buffer.backlog_separator.text {
            data::buffer::BacklogText::Hidden => row![
                container(rule::horizontal(1).style(theme::rule::backlog))
                    .padding([2, 0])
                    .width(Length::Fill)
            ]
            .padding(2)
            .align_y(iced::Alignment::Center),
            data::buffer::BacklogText::Text(separator_text) => row![
                container(rule::horizontal(1).style(theme::rule::backlog))
                    .width(Length::Fill)
                    .padding(padding::right(6)),
                text(separator_text)
                    .size(divider_font_size)
                    .style(theme::text::backlog)
                    .font_maybe(
                        theme::font_style::secondary(theme).map(font::get)
                    ),
                container(rule::horizontal(1).style(theme::rule::backlog))
                    .width(Length::Fill)
                    .padding(padding::left(6))
            ]
            .padding(2)
            .align_y(iced::Alignment::Center),
        }
    } else {
        row![]
    };

    let content = on_resize(
        column![
            top_row,
            top_spacer,
            column(old).spacing(line_spacing),
            keyed(keyed::Key::Divider, divider),
            column(new).spacing(line_spacing),
            bottom_spacer,
            space::vertical().height(line_spacing),
        ]
        .padding(padding::bottom(reserved_bottom_padding))
        .spacing(line_spacing),
        Message::ContentResized,
    );

    correct_viewport(
        Scrollable::new(container(content).width(Length::Fill).padding([0, 8]))
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::default()
                    .anchor(status.anchor())
                    .width(config.pane.scrollbar.width)
                    .scroller_width(config.pane.scrollbar.scroller_width),
            ))
            .on_scroll(move |viewport| Message::Scrolled {
                has_more_older_messages,
                has_more_newer_messages,
                count,
                oldest,
                status,
                viewport,
            })
            .id(state.scrollable.clone()),
        state.scrollable.clone(),
        matches!(state.status, Status::Unlocked),
    )
}

#[derive(Debug, Clone)]
pub struct State {
    pub scrollable: widget::Id,
    pane_size: Size,
    content_size: Size,
    limit: Limit,
    status: Status,
    last_scroll_offset: f32,
    height_cache: HashMap<keyed::Key, f32>,
    pending_scroll_to: Option<keyed::Key>,
    pending_scroll_animate: bool,
    pending_scroll_align: ScrollAnchor,
    is_scrolling_to: bool,
    highlighted_message: Option<(message::Hash, f32)>,
    hover_highlighted_message: Option<message::Hash>,
    highlight_generation: u64,
    visible_url_messages: HashMap<message::Hash, Vec<url::Url>>,
    visible_messages: HashSet<message::Hash>,
    pending_preview_exits: HashSet<message::Hash>,
    reply_preview_urls: HashMap<message::Hash, Vec<url::Url>>,
    hovered_preview: Option<(message::Hash, usize)>,
    focus_menu: Option<FocusMenu>,
    focused_link: Option<usize>,
}

impl State {
    pub fn new(pane_size: Size, config: &Config) -> Self {
        let step_messages = step_messages(2.0 * pane_size.height, config);

        Self {
            scrollable: widget::Id::unique(),
            pane_size,
            content_size: Size::default(), // Will get set initially via `on_resize`
            limit: Limit::Bottom(step_messages),
            status: Status::default(),
            last_scroll_offset: 0.0,
            height_cache: HashMap::new(),
            pending_scroll_to: None,
            pending_scroll_animate: true,
            pending_scroll_align: ScrollAnchor::default(),
            is_scrolling_to: false,
            highlighted_message: None,
            hover_highlighted_message: None,
            highlight_generation: 0,

            visible_url_messages: HashMap::new(),
            visible_messages: HashSet::new(),
            pending_preview_exits: HashSet::new(),
            reply_preview_urls: HashMap::new(),
            hovered_preview: None,
            focus_menu: None,
            focused_link: None,
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        focused_message: &mut Option<message::Hash>,
        infinite_scroll: bool,
        kind: Kind,
        buffer: Option<&buffer::Upstream>,
        history: &mut history::Manager,
        clients: &mut client::Map,
        config: &Config,
        previews: Option<&Previews>,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::Scrolled {
                count,
                has_more_older_messages,
                has_more_newer_messages,
                oldest,
                status: old_status,
                viewport,
            } => {
                if self.pending_scroll_to.is_some() || self.is_scrolling_to {
                    return (Task::none(), None);
                }

                self.last_scroll_offset = viewport.absolute_offset().y;

                let relative_offset = viewport.relative_offset().y;
                let absolute_offset = viewport.absolute_offset().y;
                let height = self.pane_size.height;

                let mut event = None;

                match old_status {
                    // Scrolling down from top & have more to load
                    _ if old_status.is_page_from_bottom(
                        absolute_offset,
                        height,
                        self.content_size.height,
                    ) && has_more_newer_messages =>
                    {
                        self.status = Status::Unlocked;
                        let n = count + step_messages(height, config);
                        self.limit = match self.limit {
                            Limit::Around(_, hash) => Limit::Around(n, hash),
                            _ => Limit::Top(n),
                        };
                    }
                    // Hit bottom, anchor it
                    _ if old_status.is_bottom(relative_offset) => {
                        if !matches!(self.status, Status::Bottom)
                            && config.buffer.mark_as_read.on_scroll_to_bottom
                        {
                            event = Some(Event::MarkAsRead);
                        }

                        self.status = Status::Bottom;

                        if matches!(self.limit, Limit::Bottom(_)) {
                            if old_status.is_page_from_top(
                                absolute_offset,
                                // Scale up page height to ensure that there
                                // isn't a simultaneous anchor flip and message
                                // load when scrolling up from bottom
                                2.0 * height,
                                self.content_size.height,
                            ) && has_more_older_messages
                            {
                                self.limit = Limit::Bottom(
                                    count + step_messages(height, config),
                                );
                            }
                        } else {
                            self.limit = Limit::Bottom(step_messages(
                                2.0 * height,
                                config,
                            ));
                        }
                    }
                    // Scrolling up from bottom & have more to load
                    _ if old_status.is_page_from_top(
                        absolute_offset,
                        height,
                        self.content_size.height,
                    ) && has_more_older_messages =>
                    {
                        self.status = Status::Unlocked;
                        let n = count + step_messages(height, config);

                        if let Limit::Around(_, hash) = self.limit {
                            self.limit = Limit::Around(n, hash);
                        } else {
                            self.limit = Limit::Bottom(n);

                            // Get new oldest message w/ new limit and use that w/ Since
                            if let Some(history::View {
                                old_messages,
                                new_messages,
                                ..
                            }) = history.get_messages(
                                &kind.into(),
                                Some(self.limit),
                                config,
                            ) && let Some(oldest) =
                                old_messages.iter().chain(&new_messages).next()
                            {
                                self.limit = Limit::Since(oldest.server_time);
                            }
                        }
                    }
                    // Hit top
                    _ if old_status.is_top(relative_offset) => {
                        // If we're infinite scroll & out of messages, load more via chathistory
                        if let Some(server) = kind.server().filter(|_| {
                            infinite_scroll && !has_more_older_messages
                        }) {
                            // Load more history & ensure scrollable is unlocked
                            event = Some(Event::RequestOlderChatHistory);
                            self.status = Status::Unlocked;
                            self.limit = Limit::Top(
                                clients.get_server_chathistory_limit(server)
                                    as usize
                                    + step_messages(height, config),
                            );
                        } else {
                            // Anchor it
                            self.status = Status::Unlocked;

                            if matches!(self.limit, Limit::Top(_)) {
                                if old_status.is_page_from_bottom(
                                    absolute_offset,
                                    height,
                                    self.content_size.height,
                                ) && has_more_newer_messages
                                {
                                    self.limit = Limit::Top(
                                        count + step_messages(height, config),
                                    );
                                }
                            } else if matches!(self.limit, Limit::Around(_, _))
                            {
                                self.limit = Limit::Since(oldest);
                            } else {
                                self.limit = Limit::Top(step_messages(
                                    2.0 * height,
                                    config,
                                ));
                            }
                        }
                    }
                    // Move away from bottom
                    Status::Bottom
                        if !old_status.is_bottom(relative_offset) =>
                    {
                        self.status = Status::Unlocked;
                        self.limit = Limit::Since(oldest);
                    }
                    // Normal scrolling, always unlocked
                    _ => {
                        self.status = Status::Unlocked;

                        if !matches!(
                            self.limit,
                            Limit::Top(_) | Limit::Around(_, _)
                        ) {
                            self.limit = Limit::Since(oldest);
                        }
                    }
                }

                // If alignment changes, we need to flip the scrollable translation
                // for the new offset
                if let Some(new_offset) =
                    self.status.flipped(old_status, viewport)
                {
                    self.last_scroll_offset = new_offset.y;
                    let scroll_to = correct_viewport::scroll_to(
                        self.scrollable.clone(),
                        new_offset,
                    );
                    let collect =
                        keyed::collect_heights(self.scrollable.clone())
                            .map(Message::HeightsCollected);

                    return (Task::batch([scroll_to, collect]), event);
                }

                let collect = keyed::collect_heights(self.scrollable.clone())
                    .map(Message::HeightsCollected);

                return (collect, event);
            }
            Message::ContextMenu(message) => {
                return (
                    Task::none(),
                    context_menu::update(message).map(Event::ContextMenu),
                );
            }
            Message::Link(message::Link::Channel(
                server,
                channel,
                buffer_action,
            )) => {
                return (
                    Task::none(),
                    Some(Event::OpenBuffer(
                        server,
                        Target::Channel(channel),
                        buffer_action,
                    )),
                );
            }
            Message::Link(message::Link::Url(url)) => {
                return (Task::none(), Some(Event::OpenUrl(url)));
            }
            Message::Link(message::Link::User(server, user)) => {
                let event = match config.actions.buffer.click_username {
                    NicknameClickAction::OpenQuery(buffer_action) => {
                        let query = target::Query::from(user);

                        Some(Event::OpenBuffer(
                            server,
                            Target::Query(query),
                            buffer_action,
                        ))
                    }
                    NicknameClickAction::InsertNickname => {
                        Some(Event::ContextMenu(
                            context_menu::Event::InsertNickname(
                                user.nickname().to_owned(),
                            ),
                        ))
                    }
                    NicknameClickAction::Noop => None,
                };

                return (Task::none(), event);
            }
            Message::Link(message::Link::GoToMessage(
                server,
                channel,
                message,
                buffer_action,
            )) => {
                return (
                    Task::none(),
                    Some(Event::GoToMessage(
                        server,
                        channel,
                        message,
                        buffer_action,
                    )),
                );
            }
            Message::ScrollTo(keyed::Hit {
                key,
                hit_bounds,
                scrollable,
            }) => {
                self.is_scrolling_to = false;

                self.pending_scroll_to = None;
                let animate = self.pending_scroll_animate;
                self.pending_scroll_animate = true;
                let align = self.pending_scroll_align;
                self.pending_scroll_align = ScrollAnchor::default();

                let fade_task = if animate {
                    if let keyed::Key::Message(hash) = key {
                        self.highlight_generation += 1;
                        let generation = self.highlight_generation;
                        self.highlighted_message =
                            Some((hash, HIGHLIGHT_ALPHA_START));
                        Task::perform(
                            time::sleep(Duration::from_millis(
                                HIGHLIGHT_HOLD_MS,
                            )),
                            move |()| Message::FadeHighlight(hash, generation),
                        )
                    } else {
                        Task::none()
                    }
                } else {
                    Task::none()
                };

                let max_offset = scrollable.max_vertical_offset();

                let content_y = hit_bounds.y - scrollable.content.y;
                let viewport_top = scrollable.offset.y;
                let viewport_bottom =
                    scrollable.offset.y + scrollable.viewport.height;
                let is_visible = content_y >= viewport_top
                    && content_y + hit_bounds.height <= viewport_bottom;

                if is_visible {
                    return (fade_task, None);
                }

                let aligned_y = match align {
                    ScrollAnchor::Top => content_y,
                    ScrollAnchor::Bottom => {
                        content_y + hit_bounds.height
                            - scrollable.viewport.height
                    }
                };
                let offset = aligned_y.max(0.0).min(max_offset);

                if (offset - max_offset).abs() <= f32::EPSILON {
                    self.status = Status::Bottom;

                    if !matches!(self.limit, Limit::Bottom(_)) {
                        self.limit = Limit::Bottom(step_messages(
                            2.0 * self.pane_size.height,
                            config,
                        ));
                    }

                    return (
                        Task::batch([
                            correct_viewport::scroll_to(
                                self.scrollable.clone(),
                                scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
                            ),
                            fade_task,
                        ]),
                        None,
                    );
                } else {
                    self.status = Status::Unlocked;

                    return (
                        Task::batch([
                            correct_viewport::scroll_to(
                                self.scrollable.clone(),
                                scrollable::AbsoluteOffset {
                                    x: 0.0,
                                    y: offset,
                                },
                            ),
                            fade_task,
                        ]),
                        None,
                    );
                }
            }
            Message::Link(message::Link::ExpandMessage(server_time, hash)) => {
                return (
                    Task::none(),
                    Some(Event::ExpandMessage(server_time, hash)),
                );
            }
            Message::Link(message::Link::ContractMessage(
                server_time,
                hash,
            )) => {
                return (
                    Task::none(),
                    Some(Event::ContractMessage(server_time, hash)),
                );
            }
            Message::RequestOlderChatHistory => {
                if let Some(server) = kind.server() {
                    self.status = Status::Unlocked;
                    self.limit = Limit::Top(
                        clients.get_server_chathistory_limit(server) as usize
                            + step_messages(self.pane_size.height, config),
                    );

                    return (
                        Task::none(),
                        Some(Event::RequestOlderChatHistory),
                    );
                }
            }
            Message::EnteringViewport(hash, urls) => {
                self.pending_preview_exits.remove(&hash);
                self.visible_url_messages.insert(hash, urls);
                return (Task::none(), Some(Event::PreviewChanged));
            }
            Message::ExitingViewport(hash) => {
                if self.visible_url_messages.contains_key(&hash) {
                    self.pending_preview_exits.insert(hash);
                }
                return (Task::none(), None);
            }
            Message::EnteredViewport(hash) => {
                self.visible_messages.insert(hash);
            }
            Message::ExitedViewport(hash) => {
                self.visible_messages.remove(&hash);
            }
            Message::ReplyPreviewHovered(hash, reply_hash, urls) => {
                if config.buffer.reply.highlight_hovered_message
                    && self.visible_messages.contains(&reply_hash)
                {
                    self.hover_highlighted_message = Some(reply_hash);
                } else {
                    self.hover_highlighted_message = None;
                    if !urls.is_empty() {
                        let prev = self.reply_preview_urls.insert(hash, urls);
                        if prev.is_none() {
                            return (Task::none(), Some(Event::PreviewChanged));
                        }
                    }
                }
            }
            Message::ReplyPreviewUnhovered(hash) => {
                self.hover_highlighted_message = None;
                if self.reply_preview_urls.remove(&hash).is_some() {
                    return (Task::none(), Some(Event::PreviewChanged));
                }
            }
            Message::PreviewHovered(hash, idx) => {
                self.hovered_preview = Some((hash, idx));
            }
            Message::PreviewUnhovered(hash, idx) => {
                // Remove if its the one currently hovered
                if self
                    .hovered_preview
                    .is_some_and(|(a, b)| a == hash && b == idx)
                {
                    self.hovered_preview = None;
                }
            }
            Message::HidePreview(message, url) => {
                return (
                    Task::none(),
                    Some(Event::HidePreview(kind.into(), message, url)),
                );
            }
            Message::MarkAsRead => {
                return (Task::none(), Some(Event::MarkAsRead));
            }
            Message::ContentResized(size) => {
                self.content_size = size;
            }
            Message::ImagePreview(image) => {
                return (Task::none(), Some(Event::ImagePreview(image)));
            }
            Message::PendingScrollTo => {
                if let Some(key) = &self.pending_scroll_to {
                    let scroll_to = keyed::find(self.scrollable.clone(), *key)
                        .map(Message::ScrollTo);

                    self.pending_scroll_to = None;
                    self.is_scrolling_to = true;

                    return (scroll_to, None);
                }
            }
            Message::FadeHighlight(hash, generation) => {
                if let Some((current_hash, alpha)) =
                    &mut self.highlighted_message
                    && *current_hash == hash
                    && generation == self.highlight_generation
                {
                    *alpha -= HIGHLIGHT_ALPHA_STEP;
                    if *alpha <= 0.0 {
                        self.highlighted_message = None;
                    } else {
                        return (
                            Task::perform(
                                time::sleep(Duration::from_millis(
                                    HIGHLIGHT_ALPHA_TICK_MS,
                                )),
                                move |()| {
                                    Message::FadeHighlight(hash, generation)
                                },
                            ),
                            None,
                        );
                    }
                }
            }
            Message::HeightsCollected(heights) => {
                for (key, height) in &heights {
                    self.height_cache.insert(*key, *height);
                }

                let mut preview_changed = false;

                if !self.pending_preview_exits.is_empty()
                    || !self.visible_messages.is_empty()
                {
                    let rendered_hashes = heights
                        .iter()
                        .filter_map(|(key, _)| {
                            if let keyed::Key::Message(hash) = key {
                                Some(*hash)
                            } else {
                                None
                            }
                        })
                        .collect::<HashSet<_>>();

                    let mut still_pending = HashSet::new();

                    for hash in self.pending_preview_exits.drain() {
                        if rendered_hashes.contains(&hash) {
                            still_pending.insert(hash);
                        } else if self
                            .visible_url_messages
                            .remove(&hash)
                            .is_some()
                        {
                            preview_changed = true;
                        }
                    }

                    self.pending_preview_exits = still_pending;

                    self.visible_messages
                        .retain(|hash| rendered_hashes.contains(hash));
                }

                let event = preview_changed.then_some(Event::PreviewChanged);

                if let Some(key) = &self.pending_scroll_to {
                    let scroll_to = keyed::find(self.scrollable.clone(), *key)
                        .map(Message::ScrollTo);

                    self.pending_scroll_to = None;
                    self.is_scrolling_to = true;

                    return (scroll_to, event);
                }

                if let Some(event) = event {
                    return (Task::none(), Some(event));
                }
            }
            Message::Reacted { msgid, text } => {
                send_reaction(clients, buffer, history, msgid, text, false);
            }
            Message::Unreacted { msgid, text } => {
                send_reaction(clients, buffer, history, msgid, text, true);
            }
            Message::NavigateFocus(direction) => {
                // Moving the focus dismisses any open action menu
                self.focus_menu = None;

                let Some(history::View {
                    old_messages,
                    new_messages,
                    ..
                }) = history.get_messages(&kind.into(), None, config)
                else {
                    return (Task::none(), None);
                };

                let all: Vec<&data::Message> = old_messages
                    .iter()
                    .copied()
                    .chain(new_messages.iter().copied())
                    .filter(|m| m.target.source().user().is_some())
                    .collect();

                if all.is_empty() {
                    return (Task::none(), None);
                }

                // The focus sequence steps through each message and then its
                // individual links before moving on to the next message
                let next: Option<(message::Hash, Option<usize>)> =
                    match *focused_message {
                        None => match direction {
                            // Entering with Up lands on the bottom-most stop -
                            // the last message's last link
                            Direction::Up => all
                                .iter()
                                .rev()
                                .find(|m| {
                                    self.visible_messages.contains(&m.hash)
                                })
                                .or_else(|| all.last())
                                .map(|m| {
                                    (
                                        m.hash,
                                        message_focus_target_count(m)
                                            .checked_sub(1),
                                    )
                                }),
                            Direction::Down => all
                                .iter()
                                .find(|m| {
                                    self.visible_messages.contains(&m.hash)
                                })
                                .or_else(|| all.first())
                                .map(|m| (m.hash, None)),
                        },
                        Some(hash) => {
                            match all.iter().position(|m| m.hash == hash) {
                                None => all.last().map(|m| (m.hash, None)),
                                Some(i) => {
                                    let links =
                                        message_focus_target_count(all[i]);
                                    match direction {
                                        Direction::Down => {
                                            match self.focused_link {
                                                None if links > 0 => {
                                                    Some((hash, Some(0)))
                                                }
                                                Some(u) if u + 1 < links => {
                                                    Some((hash, Some(u + 1)))
                                                }
                                                _ if i + 1 >= all.len() => {
                                                    // End of the last message
                                                    // — exit selection.
                                                    *focused_message = None;
                                                    self.focused_link = None;
                                                    return (
                                                        Task::none(),
                                                        Some(Event::ExitFocus),
                                                    );
                                                }
                                                _ => Some((
                                                    all[i + 1].hash,
                                                    None,
                                                )),
                                            }
                                        }
                                        Direction::Up => {
                                            match self.focused_link {
                                                Some(0) => Some((hash, None)),
                                                Some(u) => {
                                                    Some((hash, Some(u - 1)))
                                                }
                                                None if i == 0 => {
                                                    Some((hash, None))
                                                }
                                                None => {
                                                    // Previous message's last stop.
                                                    let prev = all[i - 1];
                                                    Some((
                                                    prev.hash,
                                                    message_focus_target_count(prev)
                                                        .checked_sub(1),
                                                ))
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    };

                let Some((target_hash, target_url)) = next else {
                    return (Task::none(), None);
                };

                *focused_message = Some(target_hash);
                self.focused_link = target_url;

                // Anchor the message to the edge we're moving toward, so a
                // scroll reveals it at that edge rather than snapping it to the
                // opposite side of the viewport.
                let align = match direction {
                    Direction::Up => ScrollAnchor::Top,
                    Direction::Down => ScrollAnchor::Bottom,
                };

                return (
                    self.scroll_to_message(
                        target_hash,
                        kind,
                        history,
                        config,
                        false,
                        align,
                    ),
                    None,
                );
            }
            Message::OpenFocusMenu => {
                let Some(hash) = *focused_message else {
                    return (Task::none(), None);
                };

                let Some(server) = kind.server() else {
                    return (Task::none(), None);
                };

                let Some(message) = history
                    .get_messages(&kind.into(), None, config)
                    .and_then(|view| {
                        view.old_messages
                            .iter()
                            .chain(view.new_messages.iter())
                            .find(|m| m.hash == hash)
                            .copied()
                    })
                else {
                    return (Task::none(), None);
                };

                let entries = build_focus_entries(
                    message,
                    self.focused_link,
                    server,
                    clients,
                    previews,
                    config,
                );

                self.focus_menu = Some(FocusMenu {
                    hash,
                    selection: 0,
                    content: FocusMenuContent::Message(entries),
                });

                return (Task::none(), None);
            }
            Message::OpenNickFocusMenu => {
                let Some(hash) = *focused_message else {
                    return (Task::none(), None);
                };

                let (server, channel) = match kind {
                    Kind::Channel(server, channel) => (server, Some(channel)),
                    Kind::Query(server, _) => (server, None),
                    Kind::Server(_) | Kind::Logs | Kind::Highlights => {
                        return (Task::none(), None);
                    }
                };

                let Some(message) = history
                    .get_messages(&kind.into(), None, config)
                    .and_then(|view| {
                        view.old_messages
                            .iter()
                            .chain(view.new_messages.iter())
                            .find(|m| m.hash == hash)
                            .copied()
                    })
                else {
                    return (Task::none(), None);
                };

                // The nick menu only applies to messages authored by a user.
                let Some(user) = message.target.source().user() else {
                    return (Task::none(), None);
                };

                let registry = clients.get_registry(server);

                let current_user = channel.and_then(|channel| {
                    clients.resolve_user_attributes(server, channel, user)
                });

                let our_user = channel.and_then(|channel| {
                    clients.nickname(server).and_then(|our_nick| {
                        let our_user =
                            User::from(data::user::Nick::from(our_nick));
                        clients
                            .resolve_user_attributes(server, channel, &our_user)
                    })
                });

                let entries = context_menu::Entry::user_list(
                    channel.is_some(),
                    current_user,
                    our_user,
                    config.file_transfer.enabled,
                    context_menu::has_user_metadata(user, registry, config),
                );

                let data = NickFocusData {
                    server: server.clone(),
                    channel: channel.cloned(),
                    prefix: clients
                        .get_server_prefix_or_default(server)
                        .to_vec(),
                    user: user.clone(),
                    current_user: current_user.cloned(),
                    entries,
                };

                let mut menu = FocusMenu {
                    hash,
                    selection: 0,
                    content: FocusMenuContent::Nick(data),
                };
                menu.selection = menu.first_actionable();

                self.focus_menu = Some(menu);

                return (Task::none(), None);
            }
            Message::FocusMenuMove(direction) => {
                if let Some(menu) = &mut self.focus_menu {
                    menu.move_selection(direction);
                }

                return (Task::none(), None);
            }
            Message::FocusMenuActivate(index) => {
                let Some(menu) = self.focus_menu.take() else {
                    return (Task::none(), None);
                };

                // Activating an action leaves focus mode.
                self.focused_link = None;

                match menu.content {
                    FocusMenuContent::Message(entries) => {
                        let Some(entry) = entries.into_iter().nth(index) else {
                            return (Task::none(), None);
                        };

                        return match entry.action {
                            FocusEntryAction::Message(action) => {
                                (Task::none(), Some(Event::FocusAction(action)))
                            }
                            FocusEntryAction::Context(message) => (
                                Task::none(),
                                Some(Event::FocusContextAction(message)),
                            ),
                            FocusEntryAction::Link(link) => {
                                // Re-dispatch as a link click and exit focus mode.
                                *focused_message = None;
                                (
                                    Task::done(Message::Link(link)),
                                    Some(Event::ExitFocus),
                                )
                            }
                        };
                    }
                    FocusMenuContent::Nick(data) => {
                        let context = context_menu::Context::User {
                            server: &data.server,
                            prefix: &data.prefix,
                            channel: data.channel.as_ref(),
                            registry: clients.get_registry(&data.server),
                            avatar: None,
                            user: &data.user,
                            current_user: data.current_user.as_ref(),
                        };

                        let event = data
                            .entries
                            .get(index)
                            .and_then(|entry| {
                                entry.context_message(&context, config)
                            })
                            .map(Event::FocusContextAction);

                        return (Task::none(), event);
                    }
                }
            }
            Message::FocusMenuClose => {
                self.focus_menu = None;

                return (Task::none(), None);
            }

            Message::FocusMenuDismiss => {
                self.focus_menu = None;
                self.focused_link = None;
                *focused_message = None;

                return (Task::none(), Some(Event::ExitFocus));
            }
        }
        (Task::none(), None)
    }

    pub fn has_focus_menu(&self) -> bool {
        self.focus_menu.is_some()
    }

    pub fn focus_menu(&self) -> Option<&FocusMenu> {
        self.focus_menu.as_ref()
    }

    pub fn focused_link(&self) -> Option<usize> {
        self.focused_link
    }

    pub fn close_focus_menu(&mut self) {
        self.focus_menu = None;
        self.focused_link = None;
    }

    pub fn update_pane_size(&mut self, pane_size: Size, config: &Config) {
        let step_messages = step_messages(pane_size.height, config);

        match self.limit {
            Limit::Top(x) if x < step_messages => {
                self.limit = Limit::Top(step_messages);
            }
            Limit::Bottom(x) if x < step_messages => {
                self.limit = Limit::Bottom(step_messages);
            }
            Limit::Around(x, hash) if x < step_messages => {
                self.limit = Limit::Around(step_messages, hash);
            }
            _ => {}
        }

        let width_changed = self.pane_size.width != pane_size.width;

        self.pane_size = pane_size;

        if width_changed {
            self.height_cache.clear();
        }
    }

    pub fn scroll_up_page(&mut self) -> Task<Message> {
        correct_viewport::scroll_by(
            self.scrollable.clone(),
            self.status.anchor(),
            |bounds| scrollable::AbsoluteOffset {
                x: 0.0,
                y: -(bounds.height - 20.0).max(0.0).min(bounds.height),
            },
        )
    }

    pub fn scroll_down_page(&mut self) -> Task<Message> {
        correct_viewport::scroll_by(
            self.scrollable.clone(),
            self.status.anchor(),
            |bounds| scrollable::AbsoluteOffset {
                x: 0.0,
                y: (bounds.height - 20.0).max(0.0).min(bounds.height),
            },
        )
    }

    pub fn scroll_to_start(&mut self, config: &Config) -> Task<Message> {
        self.status = Status::Unlocked;
        self.limit =
            Limit::Top(step_messages(2.0 * self.pane_size.height, config));
        correct_viewport::scroll_to(
            self.scrollable.clone(),
            scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
        )
    }

    pub fn scroll_to_end(&mut self, config: &Config) -> Task<Message> {
        self.status = Status::Bottom;
        self.limit =
            Limit::Bottom(step_messages(2.0 * self.pane_size.height, config));
        correct_viewport::scroll_to(
            self.scrollable.clone(),
            scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
        )
    }

    pub fn is_scrolled_to_bottom(&self) -> bool {
        matches!(self.status, Status::Bottom)
    }

    pub fn scroll_to_message(
        &mut self,
        message: message::Hash,
        kind: Kind,
        history: &history::Manager,
        config: &Config,
        animate: bool,
        align: ScrollAnchor,
    ) -> Task<Message> {
        let Some(history::View {
            old_messages,
            new_messages,
            ..
        }) = history.get_messages(&kind.into(), None, config)
        else {
            // We're still loading history, which will trigger scroll_to_backlog
            // after loading. If this is set, we will scroll_to_message
            self.pending_scroll_to = Some(keyed::Key::Message(message));
            self.pending_scroll_animate = animate;
            self.pending_scroll_align = align;

            return Task::none();
        };

        let Some(target) = old_messages
            .iter()
            .chain(&new_messages)
            .find(|m| m.hash == message)
        else {
            return Task::none();
        };

        self.pending_scroll_animate = animate;
        self.pending_scroll_align = align;

        // If the message is already rendered, skip the load and fire immediately.
        if self
            .height_cache
            .contains_key(&keyed::Key::Message(message))
        {
            self.is_scrolling_to = true;

            // cache real heights while fully rendered so the virtualized
            // layout's doesn't drift from estimates as focus moves.
            // without this, the error increases over time which leads to
            // unpredictable scrolling.
            let find = keyed::find(
                self.scrollable.clone(),
                keyed::Key::Message(message),
            )
            .map(Message::ScrollTo);

            // only do this when something is unmeasured — in steady state every
            // height is already cached and re-collecting would be wasted work.
            let needs_heights =
                old_messages.iter().chain(&new_messages).any(|m| {
                    !self
                        .height_cache
                        .contains_key(&keyed::Key::Message(m.hash))
                });

            return if needs_heights {
                Task::batch([
                    keyed::collect_heights(self.scrollable.clone())
                        .map(Message::HeightsCollected),
                    find,
                ])
            } else {
                find
            };
        }

        // Load a window of messages centered on the target.
        let around_count = step_messages(4.0 * self.pane_size.height, config);
        self.limit = Limit::Around(around_count, target.hash);

        self.pending_scroll_to = Some(keyed::Key::Message(message));

        Task::perform(time::sleep(SCROLL_TO_TIMEOUT), move |()| {
            Message::PendingScrollTo
        })
    }

    pub fn scroll_to_backlog(
        &mut self,
        kind: Kind,
        history: &history::Manager,
        config: &Config,
    ) -> Task<Message> {
        if self.pending_scroll_to.is_some() {
            return Task::perform(time::sleep(SCROLL_TO_TIMEOUT), move |()| {
                Message::PendingScrollTo
            });
        }

        let Some(history::View {
            old_messages,
            new_messages,
            ..
        }) = history.get_messages(&kind.into(), None, config)
        else {
            return Task::none();
        };

        if new_messages.is_empty() {
            return self.scroll_to_end(config);
        }

        // Use the message at the divider boundary as anchor
        let Some(target) = old_messages
            .iter()
            .chain(&new_messages)
            .nth(old_messages.len().saturating_sub(1))
        else {
            return Task::none();
        };

        let around_count = step_messages(4.0 * self.pane_size.height, config);
        self.limit = Limit::Around(around_count, target.hash);

        self.pending_scroll_to = Some(keyed::Key::Divider);

        Task::perform(time::sleep(SCROLL_TO_TIMEOUT), move |()| {
            Message::PendingScrollTo
        })
    }

    pub fn has_pending_scroll_to(&self) -> bool {
        self.pending_scroll_to.is_some()
    }

    pub fn prepare_for_pending_scroll_to(
        &mut self,
        kind: Kind,
        history: &history::Manager,
        config: &Config,
    ) -> Task<Message> {
        let Some(key) = self.pending_scroll_to else {
            return Task::none();
        };

        let Some(history::View {
            old_messages,
            new_messages,
            ..
        }) = history.get_messages(&kind.into(), None, config)
        else {
            return Task::none();
        };

        let around_count = step_messages(4.0 * self.pane_size.height, config);

        match key {
            keyed::Key::Message(message) | keyed::Key::Preview(message, _) => {
                let Some(target) = old_messages
                    .iter()
                    .chain(&new_messages)
                    .find(|m| m.hash == message)
                else {
                    return Task::none();
                };

                // Load a window of messages centered on the target
                self.limit = Limit::Around(around_count, target.hash);
            }
            keyed::Key::Divider => {
                let Some(target) = old_messages
                    .iter()
                    .chain(&new_messages)
                    .nth(old_messages.len().saturating_sub(1))
                else {
                    return Task::none();
                };

                self.limit = Limit::Around(around_count, target.hash);
            }
        };

        keyed::collect_heights(self.scrollable.clone())
            .map(Message::HeightsCollected)
    }

    pub fn visible_urls(&self) -> impl Iterator<Item = &url::Url> {
        self.visible_url_messages
            .values()
            .flatten()
            .chain(self.reply_preview_urls.values().flatten())
    }
}

fn send_reaction(
    clients: &mut client::Map,
    buffer: Option<&buffer::Upstream>,
    history: &mut history::Manager,
    msgid: message::Id,
    text: Cow<'static, str>,
    unreact: bool,
) -> Option<()> {
    let buffer = buffer?;
    let server = buffer.server();
    let target = buffer.target()?;
    let command = match unreact {
        true => Irc::Unreact {
            target: target.to_string(),
            msgid: msgid.clone(),
            text: text.clone(),
        },
        false => Irc::React {
            target: target.to_string(),
            msgid: msgid.clone(),
            text: text.clone(),
        },
    };

    let encoded = message::Encoded::try_from(command).ok()?;
    let labeled_response_context =
        clients.send(buffer, encoded, TokenPriority::User);

    if !clients.get_server_supports_echoes(server) {
        let nick = clients.nickname(server)?;
        history.record_reaction(
            server,
            reaction::Context {
                inner: Reaction {
                    sender: nick.to_owned(),
                    text: text.into_owned(),
                    unreact,
                    id: labeled_response_context
                        .map(|context| context.label_as_id),
                    server_time: Utc::now(),
                },
                target,
                in_reply_to: msgid,
                is_echo: false,
                deduplicate: false,
            },
            false,
            None,
        );
    }

    Some(())
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Status {
    #[default]
    Bottom,
    Unlocked,
}

impl Status {
    fn anchor(self) -> scrollable::Anchor {
        match self {
            Status::Bottom => scrollable::Anchor::End,
            Status::Unlocked => scrollable::Anchor::Start,
        }
    }

    fn is_top(self, relative_offset: f32) -> bool {
        match self.anchor() {
            scrollable::Anchor::Start => relative_offset == 0.0,
            scrollable::Anchor::End => relative_offset == 1.0,
        }
    }

    fn is_bottom(self, relative_offset: f32) -> bool {
        match self.anchor() {
            scrollable::Anchor::Start => relative_offset == 1.0,
            scrollable::Anchor::End => relative_offset == 0.0,
        }
    }

    fn is_page_from_top(
        self,
        absolute_offset: f32,
        page_height: f32,
        content_height: f32,
    ) -> bool {
        match self.anchor() {
            scrollable::Anchor::Start => absolute_offset <= page_height,
            scrollable::Anchor::End => {
                absolute_offset >= content_height - 2.0 * page_height
            }
        }
    }

    fn is_page_from_bottom(
        self,
        absolute_offset: f32,
        page_height: f32,
        content_height: f32,
    ) -> bool {
        match self.anchor() {
            scrollable::Anchor::Start => {
                absolute_offset >= content_height - 2.0 * page_height
            }
            scrollable::Anchor::End => absolute_offset <= page_height,
        }
    }

    fn flipped(
        self,
        other: Self,
        viewport: scrollable::Viewport,
    ) -> Option<scrollable::AbsoluteOffset> {
        if self.anchor() != other.anchor() {
            let offset = viewport.absolute_offset();
            let reversed_offset = viewport.absolute_offset_reversed();

            Some(scrollable::AbsoluteOffset {
                x: offset.x,
                y: reversed_offset.y,
            })
        } else {
            None
        }
    }
}

fn step_messages(height: f32, config: &Config) -> usize {
    let line_height = theme::resolve_line_height(&config.font);

    (height / line_height).max(8.0) as usize
}

pub mod keyed {
    use data::message;
    use iced::advanced::widget::{self, Operation};
    use iced::widget::scrollable::{self, AbsoluteOffset};
    use iced::{Rectangle, Task, Vector, advanced};

    use crate::widget::{Element, Renderer, decorate};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Key {
        Divider,
        Message(message::Hash),
        Preview(message::Hash, usize),
    }

    impl Key {
        pub fn message(message: &data::Message) -> Self {
            Self::Message(message.hash)
        }
    }

    pub fn keyed<'a, Message: 'a>(
        key: Key,
        inner: impl Into<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        decorate(inner)
            .operate(
                move |_state: &mut (),
                      inner: &mut Element<'a, Message>,
                      tree: &mut advanced::widget::Tree,
                      layout: advanced::Layout<'_>,
                      renderer: &Renderer,
                      operation: &mut dyn advanced::widget::Operation<()>| {
                    let mut key = key;
                    operation.custom(None, layout.bounds(), &mut key);
                    inner.as_widget_mut().operate(tree, layout, renderer, operation);
                },
            )
            .into()
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Hit {
        pub key: Key,
        pub hit_bounds: Rectangle,
        pub scrollable: Scrollable,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Scrollable {
        pub viewport: Rectangle,
        pub content: Rectangle,
        pub offset: AbsoluteOffset,
    }

    impl Scrollable {
        pub fn max_vertical_offset(&self) -> f32 {
            (self.content.height - self.viewport.height).max(0.0)
        }

        pub fn reversed_offset(&self) -> AbsoluteOffset {
            AbsoluteOffset {
                x: (self.content.width - self.viewport.width).max(0.0)
                    - self.offset.x,
                y: (self.content.height - self.viewport.height).max(0.0)
                    - self.offset.y,
            }
        }
    }

    impl From<scrollable::Viewport> for Scrollable {
        fn from(viewport: scrollable::Viewport) -> Self {
            Self {
                viewport: viewport.bounds(),
                content: viewport.content_bounds(),
                offset: viewport.absolute_offset(),
            }
        }
    }

    pub fn find(scrollable: widget::Id, key: Key) -> Task<Hit> {
        widget::operate(Find {
            active: false,
            scrollable_id: scrollable,
            key,
            scrollable: None,
            hit_bounds: None,
        })
    }

    #[derive(Debug, Clone)]
    pub struct Find {
        pub active: bool,
        pub key: Key,
        pub scrollable_id: widget::Id,
        pub scrollable: Option<Scrollable>,
        pub hit_bounds: Option<Rectangle>,
    }

    impl Operation<Hit> for Find {
        fn scrollable(
            &mut self,
            id: Option<&widget::Id>,
            bounds: Rectangle,
            content_bounds: Rectangle,
            translation: Vector,
            _state: &mut dyn widget::operation::Scrollable,
        ) {
            if id.is_some_and(|id| *id == self.scrollable_id) {
                self.scrollable = Some(Scrollable {
                    viewport: bounds,
                    content: content_bounds,
                    offset: AbsoluteOffset {
                        x: translation.x,
                        y: translation.y,
                    },
                });
                self.active = true;
            } else {
                self.active = false;
            }
        }

        fn container(&mut self, _id: Option<&widget::Id>, _bounds: Rectangle) {}

        fn traverse(
            &mut self,
            operate: &mut dyn FnMut(&mut dyn Operation<Hit>),
        ) {
            operate(self);
        }

        fn custom(
            &mut self,
            _id: Option<&widget::Id>,
            bounds: Rectangle,
            state: &mut dyn std::any::Any,
        ) {
            if self.active
                && let Some(key) = state.downcast_ref::<Key>()
                && self.key == *key
            {
                self.hit_bounds = Some(bounds);
            }
        }

        fn finish(&self) -> widget::operation::Outcome<Hit> {
            match self.scrollable.zip(self.hit_bounds).map(
                |(scrollable, hit_bounds)| Hit {
                    key: self.key,
                    scrollable,
                    hit_bounds,
                },
            ) {
                Some(hit) => widget::operation::Outcome::Some(hit),
                None => widget::operation::Outcome::None,
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct TopOfViewport {
        pub active: bool,
        pub scrollable_id: widget::Id,
        pub scrollable: Option<Scrollable>,
        pub hit_bounds: Option<(Key, Rectangle)>,
    }

    impl Operation<Hit> for TopOfViewport {
        fn scrollable(
            &mut self,
            id: Option<&widget::Id>,
            bounds: Rectangle,
            content_bounds: Rectangle,
            translation: Vector,
            _state: &mut dyn widget::operation::Scrollable,
        ) {
            if id.is_some_and(|id| *id == self.scrollable_id) {
                self.scrollable = Some(Scrollable {
                    viewport: bounds,
                    content: content_bounds,
                    offset: AbsoluteOffset {
                        x: translation.x,
                        y: translation.y,
                    },
                });
                self.active = true;
            } else {
                self.active = false;
            }
        }

        fn container(&mut self, _id: Option<&widget::Id>, _bounds: Rectangle) {}

        fn traverse(
            &mut self,
            operate: &mut dyn FnMut(&mut dyn Operation<Hit>),
        ) {
            operate(self);
        }

        fn custom(
            &mut self,
            _id: Option<&widget::Id>,
            bounds: Rectangle,
            state: &mut dyn std::any::Any,
        ) {
            if self.active
                && let Some(key) = state.downcast_ref::<Key>()
                && self.hit_bounds.is_none()
                && self.scrollable.is_some_and(|scrollable| {
                    scrollable.viewport.intersects(
                        &(bounds
                            - Vector::new(
                                scrollable.offset.x,
                                scrollable.offset.y,
                            )),
                    )
                })
            {
                self.hit_bounds = Some((*key, bounds));
            }
        }

        fn finish(&self) -> widget::operation::Outcome<Hit> {
            match self.scrollable.zip(self.hit_bounds).map(
                |(scrollable, (key, hit_bounds))| Hit {
                    key,
                    scrollable,
                    hit_bounds,
                },
            ) {
                Some(hit) => widget::operation::Outcome::Some(hit),
                None => widget::operation::Outcome::None,
            }
        }
    }

    pub struct CollectHeights {
        active: bool,
        scrollable_id: widget::Id,
        heights: Vec<(Key, f32)>,
    }

    impl Operation<Vec<(Key, f32)>> for CollectHeights {
        fn scrollable(
            &mut self,
            id: Option<&widget::Id>,
            _bounds: Rectangle,
            _content_bounds: Rectangle,
            _translation: Vector,
            _state: &mut dyn widget::operation::Scrollable,
        ) {
            self.active = id == Some(&self.scrollable_id);
        }

        fn container(&mut self, _id: Option<&widget::Id>, _bounds: Rectangle) {}

        fn traverse(
            &mut self,
            operate: &mut dyn FnMut(&mut dyn Operation<Vec<(Key, f32)>>),
        ) {
            operate(self);
        }

        fn custom(
            &mut self,
            _id: Option<&widget::Id>,
            bounds: Rectangle,
            state: &mut dyn std::any::Any,
        ) {
            if self.active
                && let Some(key) = state.downcast_ref::<Key>()
                && matches!(key, Key::Message(_) | Key::Divider)
            {
                self.heights.push((*key, bounds.height));
            }
        }

        fn finish(&self) -> widget::operation::Outcome<Vec<(Key, f32)>> {
            if self.heights.is_empty() {
                widget::operation::Outcome::None
            } else {
                widget::operation::Outcome::Some(self.heights.clone())
            }
        }
    }

    pub fn collect_heights(scrollable: widget::Id) -> Task<Vec<(Key, f32)>> {
        widget::operate(CollectHeights {
            active: false,
            scrollable_id: scrollable,
            heights: vec![],
        })
    }
}

mod correct_viewport {
    use std::any::Any;
    use std::sync::{Arc, Mutex};

    use iced::advanced::widget::operation::{Scrollable, scrollable};
    use iced::advanced::widget::{Id, Operation};
    use iced::advanced::{self, widget};
    use iced::widget::scrollable::{AbsoluteOffset, Anchor};
    use iced::{Rectangle, Task, Vector};

    use super::{Message, keyed};
    use crate::widget::{Element, Renderer, decorate};

    pub fn correct_viewport<'a>(
        inner: impl Into<Element<'a, Message>>,
        scrollable: iced::widget::Id,
        enabled: bool,
    ) -> Element<'a, Message> {
        decorate(inner)
            .update({
                let scrollable = scrollable.clone();
                move |state: &mut Option<keyed::Hit>,
                      inner: &mut Element<'a, Message>,
                      tree: &mut advanced::widget::Tree,
                      event: &iced::Event,
                      layout: advanced::Layout<'_>,
                      cursor: advanced::mouse::Cursor,
                      renderer: &Renderer,
                      clipboard: &mut dyn advanced::Clipboard,
                      shell: &mut advanced::Shell<'_, Message>,
                      viewport: &iced::Rectangle| {
                    let is_redraw = matches!(
                        event,
                        iced::Event::Window(iced::window::Event::RedrawRequested(_))
                    );

                    // Check if top-of-viewport element has shifted since we
                    // last scrolled and adjust
                    if let (true, true, Some(old)) = (enabled, is_redraw, &state) {
                        let hit = Arc::new(Mutex::new(None));

                        let mut operation = widget::operation::map(
                            keyed::Find {
                                active: false,
                                key: old.key,
                                scrollable_id: scrollable.clone(),
                                scrollable: None,
                                hit_bounds: None,
                            },
                            {
                                let hit = hit.clone();
                                move |result| {
                                    *hit.lock().unwrap() = Some(result);
                                }
                            },
                        );

                        inner
                            .as_widget_mut()
                            .operate(tree, layout, renderer, &mut operation);
                        operation.finish();
                        drop(operation);

                        if let Some(new) = Arc::into_inner(hit)
                            .and_then(|m| m.into_inner().ok())
                            .flatten()
                        {
                            // Something shifted this, let's put it back to the
                            // top of the viewport
                            if new.hit_bounds.y != old.hit_bounds.y {
                                let viewport_offset = old.scrollable.viewport.y
                                    - (old.hit_bounds.y - old.scrollable.offset.y);

                                // New offset needed to place same element back to same offset
                                // from top of viewport
                                let new_offset = f32::min(
                                    (new.hit_bounds.y + viewport_offset)
                                        - new.scrollable.viewport.y,
                                    new.scrollable.content.height - new.scrollable.viewport.height,
                                );

                                let mut operation = scrollable::scroll_to(
                                    scrollable.clone(),
                                    scrollable::AbsoluteOffset {
                                        x: None,
                                        y: Some(new_offset),
                                    },
                                );
                                inner
                                    .as_widget_mut()
                                    .operate(tree, layout, renderer, &mut operation);
                                operation.finish();
                            }
                        }
                    }

                    let mut messages = vec![];
                    let mut local_shell = advanced::Shell::new(&mut messages);

                    inner.as_widget_mut().update(
                        tree,
                        event,
                        layout,
                        cursor,
                        renderer,
                        clipboard,
                        &mut local_shell,
                        viewport,
                    );

                    // Merge shell (we can't use Shell::merge as we'd lose
                    // access to messages)
                    {
                        match local_shell.redraw_request() {
                            iced::window::RedrawRequest::NextFrame => shell.request_redraw(),
                            iced::window::RedrawRequest::At(at) => shell.request_redraw_at(at),
                            iced::window::RedrawRequest::Wait => {}
                        }

                        if local_shell.is_layout_invalid() {
                            shell.invalidate_layout();
                        }

                        if local_shell.are_widgets_invalid() {
                            shell.invalidate_widgets();
                        }

                        if local_shell.is_event_captured() {
                            shell.capture_event();
                        }
                    }

                    let mut is_scrolled = false;
                    for message in messages {
                        is_scrolled |=
                            matches!(message, Message::Scrolled { .. });
                        shell.publish(message);
                    }

                    // Re-query top of viewport any-time we scroll
                    if is_scrolled {
                        let hit = Arc::new(Mutex::new(None));

                        let mut operation = widget::operation::map(
                            keyed::TopOfViewport {
                                active: false,
                                scrollable_id: scrollable.clone(),
                                scrollable: None,
                                hit_bounds: None,
                            },
                            {
                                let hit = hit.clone();
                                move |result| {
                                    *hit.lock().unwrap() = Some(result);
                                }
                            },
                        );

                        inner
                            .as_widget_mut()
                            .operate(tree, layout, renderer, &mut operation);
                        operation.finish();
                        drop(operation);

                        *state = Arc::into_inner(hit)
                            .and_then(|m| m.into_inner().ok())
                            .flatten();
                    }
                }
            })
            .operate(
                move |state: &mut Option<keyed::Hit>,
                      inner: &mut Element<'a, Message>,
                      tree: &mut advanced::widget::Tree,
                      layout: advanced::Layout<'_>,
                      renderer: &Renderer,
                      operation: &mut dyn advanced::widget::Operation<()>| {
                    inner.as_widget_mut().operate(tree, layout, renderer, operation);

                    let mut is_scroll_to = false;

                    operation.custom(
                        Some(&scrollable),
                        layout.bounds(),
                        &mut is_scroll_to,
                    );

                    if is_scroll_to {
                        let hit = Arc::new(Mutex::new(None));

                        let mut operation = widget::operation::map(
                            keyed::TopOfViewport {
                                active: false,
                                scrollable_id: scrollable.clone(),
                                scrollable: None,
                                hit_bounds: None,
                            },
                            {
                                let hit = hit.clone();
                                move |result| {
                                    *hit.lock().unwrap() = Some(result);
                                }
                            },
                        );

                        inner
                            .as_widget_mut()
                            .operate(tree, layout, renderer, &mut operation);
                        operation.finish();
                        drop(operation);

                        *state = Arc::into_inner(hit)
                            .and_then(|m| m.into_inner().ok())
                            .flatten();
                    }
                },
            )
            .into()
    }

    pub fn scroll_to<T: Send + 'static>(
        target: impl Into<Id>,
        offset: AbsoluteOffset,
    ) -> Task<T> {
        struct ScrollTo {
            target: Id,
            offset: AbsoluteOffset,
        }

        impl<T> Operation<T> for ScrollTo {
            fn container(&mut self, _id: Option<&Id>, _bounds: Rectangle) {}

            fn traverse(
                &mut self,
                operate: &mut dyn FnMut(&mut dyn Operation<T>),
            ) {
                operate(self);
            }

            fn scrollable(
                &mut self,
                id: Option<&Id>,
                _bounds: Rectangle,
                _content_bounds: Rectangle,
                _translation: Vector,
                state: &mut dyn Scrollable,
            ) {
                if id.is_some_and(|id| *id == self.target) {
                    state.scroll_to(self.offset.into());
                }
            }

            fn custom(
                &mut self,
                id: Option<&Id>,
                _bounds: Rectangle,
                state: &mut dyn Any,
            ) {
                if id.is_some_and(|id| *id == self.target)
                    && let Some(is_scroll_to) = state.downcast_mut::<bool>()
                {
                    *is_scroll_to = true;
                }
            }
        }

        widget::operate(ScrollTo {
            target: target.into(),
            offset,
        })
    }

    pub fn scroll_by<T: Send + 'static>(
        target: impl Into<Id>,
        anchor: Anchor,
        f: impl Fn(Rectangle) -> AbsoluteOffset + Send + 'static,
    ) -> Task<T> {
        struct ScrollBy {
            target: Id,
            anchor: Anchor,
            f: Box<dyn Fn(Rectangle) -> AbsoluteOffset + Send>,
        }

        impl<T> Operation<T> for ScrollBy {
            fn container(&mut self, _id: Option<&Id>, _bounds: Rectangle) {}

            fn traverse(
                &mut self,
                operate: &mut dyn FnMut(&mut dyn Operation<T>),
            ) {
                operate(self);
            }

            fn scrollable(
                &mut self,
                id: Option<&Id>,
                bounds: Rectangle,
                content_bounds: Rectangle,
                _translation: Vector,
                state: &mut dyn Scrollable,
            ) {
                if Some(&self.target) == id {
                    let mut offset = (self.f)(bounds);

                    // Flip offset
                    if matches!(self.anchor, Anchor::End) {
                        offset.y = -offset.y;
                    }

                    state.scroll_by(offset, bounds, content_bounds);
                }
            }

            fn custom(
                &mut self,
                id: Option<&Id>,
                _bounds: Rectangle,
                state: &mut dyn Any,
            ) {
                if id.is_some_and(|id| *id == self.target)
                    && let Some(is_scroll_to) = state.downcast_mut::<bool>()
                {
                    *is_scroll_to = true;
                }
            }
        }

        widget::operate(ScrollBy {
            target: target.into(),
            anchor,
            f: Box::new(f),
        })
    }
}

fn prefixes_width(message: &data::Message, config: &Config) -> Option<f32> {
    message.target.prefixes().map(|prefixes| {
        font::width_from_str(
            &format!(
                "{} ",
                config
                    .buffer
                    .status_message_prefix
                    .brackets
                    .format(prefixes.iter().collect::<String>())
            ),
            &config.font,
        ) + 1.0
    })
}

/// A keyboard-focusable link target within a message: a URL or a channel
/// mention.
#[derive(Debug, Clone)]
pub(crate) enum FocusTarget {
    Url(url::Url),
    Channel(String),
}

/// The URL of a message whose entire content is a single URL, if any.
///
/// Such a message is already selectable as a whole, so it gets no separate
/// link target; its link actions are folded into the message focus menu.
pub(crate) fn message_single_url(message: &data::Message) -> Option<url::Url> {
    let data::message::Content::Fragments(fragments) = &message.content else {
        return None;
    };

    let urls = message.content.urls();
    let url = urls.first()?;

    (urls.len() == 1
        && fragments
            .iter()
            .all(|f| f.url().is_some() || f.as_str().trim().is_empty()))
    .then(|| (*url).clone())
}

/// Iterator over a message's focusable link fragments, in display order.
fn message_focus_target_fragments(
    message: &data::Message,
) -> impl Iterator<Item = &message::Fragment> {
    let fragments: &[message::Fragment] = match &message.content {
        data::message::Content::Fragments(fragments) => fragments,
        _ => &[],
    };

    fragments.iter().filter(|f| f.is_focus_target())
}

/// Number of separately-navigable link targets in a message, in display order.
fn message_focus_target_count(message: &data::Message) -> usize {
    if message_single_url(message).is_some() {
        return 0;
    }

    message_focus_target_fragments(message).count()
}

/// The `index`-th focusable link target of a message, in display order.
pub(crate) fn message_focus_target_at(
    message: &data::Message,
    index: usize,
) -> Option<FocusTarget> {
    message_focus_target_fragments(message)
        .nth(index)
        .and_then(|fragment| match fragment {
            message::Fragment::Url(url, _) => {
                Some(FocusTarget::Url(url.clone()))
            }
            message::Fragment::Channel(channel) => {
                Some(FocusTarget::Channel(channel.clone()))
            }
            _ => None,
        })
}

fn timestamp_width(message: &data::Message, config: &Config) -> Option<f32> {
    config
        .buffer
        .format_timestamp(&message.server_time)
        .map(|timestamp| font::width_from_str(&timestamp, &config.font) + 1.0)
}
