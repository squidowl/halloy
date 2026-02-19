use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Duration;

use chrono::{DateTime, Local, NaiveDate, NaiveTime, Utc};
use data::buffer::DateSeparators;
use data::command::Irc;
use data::config::buffer::nickname::HideConsecutiveEnabled;
use data::dashboard::BufferAction;
use data::isupport::ChatHistoryState;
use data::message::{self, Limit};
use data::preview::{self, Previews};
use data::rate_limit::TokenPriority;
use data::reaction::Reaction;
use data::server::Server;
use data::target::{self, Target};
use data::{Config, Preview, client, history, reaction};
use iced::widget::{
    self, Scrollable, button, center, column, container, image, mouse_area,
    right, row, rule, scrollable, space, stack, text,
};
use iced::{ContentFit, Length, Padding, Size, Task, alignment, padding};
use tokio::time;

use self::correct_viewport::correct_viewport;
use self::keyed::keyed;
use super::context_menu;
use crate::widget::{
    Element, notify_visibility, on_resize, selectable_text, tooltip,
};
use crate::{Theme, buffer, font, icon, theme};

const HIDE_BUTTON_WIDTH: f32 = 22.0;
const SCROLL_TO_TIMEOUT: Duration = Duration::from_millis(200);
/// Pages of off-screen messages to keep rendered above and below the viewport
const BUFFER_PAGES: usize = 3;

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
    ImagePreview(PathBuf, url::Url),
    ScrollTo(keyed::Hit),
    RequestOlderChatHistory,
    EnteringViewport(message::Hash, Vec<url::Url>),
    ExitingViewport(message::Hash),
    PreviewHovered(message::Hash, usize),
    PreviewUnhovered(message::Hash, usize),
    HidePreview(message::Hash, url::Url),
    MarkAsRead,
    ContentResized(Size),
    PendingScrollTo,
    HeightsCollected(Vec<(message::Hash, f32)>),
    Reacted {
        msgid: message::Id,
        text: String,
    },
    Unreacted {
        msgid: message::Id,
        text: String,
    },
}

#[derive(Debug, Clone)]
pub enum Event {
    ContextMenu(context_menu::Event),
    OpenBuffer(Server, Target, BufferAction),
    GoToMessage(Server, target::Channel, message::Hash),
    RequestOlderChatHistory,
    PreviewChanged,
    HidePreview(history::Kind, message::Hash, url::Url),
    MarkAsRead,
    OpenUrl(String),
    ImagePreview(PathBuf, url::Url),
    ExpandCondensedMessage(DateTime<Utc>, message::Hash),
    ContractCondensedMessage(DateTime<Utc>, message::Hash),
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
    fn server(&self) -> Option<&Server> {
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
        right_aligned_width: Option<f32>,
        max_prefix_width: Option<f32>,
        range_timestamp_excess_width: Option<f32>,
        hide_nickname: bool,
    ) -> Option<Element<'a, Message>>;
}

impl<'a, T> LayoutMessage<'a> for T
where
    T: Fn(
        &'a data::Message,
        Option<f32>,
        Option<f32>,
        Option<f32>,
        bool,
    ) -> Option<Element<'a, Message>>,
{
    fn format(
        &self,
        message: &'a data::Message,
        right_aligned_width: Option<f32>,
        max_prefix_width: Option<f32>,
        range_timestamp_excess_width: Option<f32>,
        hide_nickname: bool,
    ) -> Option<Element<'a, Message>> {
        self(
            message,
            right_aligned_width,
            max_prefix_width,
            range_timestamp_excess_width,
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

pub fn view<'a>(
    state: &State,
    kind: Kind,
    history: &'a history::Manager,
    previews: Option<Previews<'a>>,
    visible_for_source: Option<impl Fn(&Preview, &message::Source) -> bool>,
    chathistory_state: Option<ChatHistoryState>,
    config: &'a Config,
    theme: &'a Theme,
    formatter: impl LayoutMessage<'a> + 'a,
) -> Element<'a, Message> {
    let divider_font_size =
        config.font.size.map_or(theme::TEXT_SIZE, f32::from) - 1.0;

    let Some(history::View {
        has_more_older_messages,
        has_more_newer_messages,
        old_messages,
        new_messages,
        max_nick_chars,
        max_prefix_chars,
        range_timestamp_extra_chars,
        cleared,
        ..
    }) = history.get_messages(&kind.into(), Some(state.limit), &config.buffer)
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

    let right_aligned_width = max_nick_chars.map(|max_nick_chars| {
        let max_nick_width =
            font::width_from_chars(max_nick_chars, &config.font);
        let message_marker_width = font::width_of_message_marker(&config.font);
        let range_timestamp_extra_width = range_timestamp_extra_chars.map_or(
            0.0,
            |range_timestamp_extra_chars| {
                font::width_from_chars(
                    range_timestamp_extra_chars,
                    &config.font,
                )
            },
        );

        max_nick_width.max(range_timestamp_extra_width + message_marker_width)
    });

    let max_prefix_width =
        max_prefix_chars.map(|len| font::width_from_chars(len, &config.font));

    let range_timestamp_excess_width = range_timestamp_extra_chars
        .map(|len| font::width_from_chars(len, &config.font));

    let message_rows = |last_date: Option<NaiveDate>,
                        messages: &[&'a data::Message]| {
        messages
            .iter()
            .scan(Option::<&data::Message>::None, |prev_message, message| {
                let hide_nickname =
                    if let HideConsecutiveEnabled::Enabled(duration) =
                        config.buffer.nickname.hide_consecutive.enabled
                    {
                        !config.buffer.nickname.alignment.is_top() &&
                        matches!(message.target.source(), message::Source::User(_)) &&
                        // don't hide if prev message has visible preview (when show_after_previews is enabled)
                        !(config.buffer.nickname.hide_consecutive.show_after_previews
                                    && prev_message.is_some_and(|prev_msg| {
                                        has_visible_preview(
                                            prev_msg,
                                            state,
                                            previews,
                                            &visible_for_source,
                                        )
                            })) &&
                        prev_message.is_some_and(|prev_message| {
                            matches!(
                                (message.target.source(), prev_message.target.source()),
                                (message::Source::User(user), message::Source::User(prev_user)) if user == prev_user
                            ) && duration.is_none_or(|duration| message.server_time - prev_message.server_time < duration)
                        })
                    } else {
                        false
                    };

                *prev_message = Some(message);

                Some(formatter
                    .format(
                        message,
                        right_aligned_width,
                        max_prefix_width,
                        range_timestamp_excess_width,
                        hide_nickname,
                    )
                    .map(|element| (message, element)))
            })
            .flatten()
            .scan(last_date, |last_date, (message, element)| {
                let date =
                    message.server_time.with_timezone(&Local).date_naive();

                let is_new_day = last_date.is_none_or(|prev| date > prev);

                *last_date = Some(date);

                let content = if let (
                    message::Content::Fragments(fragments),
                    Some(previews)
                ) =
                    (&message.content, previews)
                {
                    let urls = fragments
                        .iter()
                        .filter_map(message::Fragment::url)
                        .cloned()
                        .collect::<Vec<_>>();

                    if !urls.is_empty() {
                        let is_message_visible = state
                                    .visible_url_messages
                                    .contains_key(&message.hash);

                        let mut column = column![element].spacing(2);

                        for (idx, url) in urls.iter().enumerate() {
                            if message.hidden_urls.contains(url) {
                                continue;
                            }

                            if let (
                                true,
                                Some(preview::State::Loaded(preview)),
                            ) = (is_message_visible, previews.get(url))
                            {
                                let is_hovered =
                                    state.hovered_preview.is_some_and(
                                        |(a, b)| a == message.hash && b == idx,
                                    );

                                let is_visible_for_source =
                                    if let Some(visible_for_source) = &visible_for_source {
                                        visible_for_source(preview, message.target.source())
                                    } else {
                                        true
                                    };

                                if is_visible_for_source {
                                    column = column.push(preview_row(
                                        message,
                                        preview,
                                        url,
                                        idx,
                                        right_aligned_width,
                                        max_prefix_width,
                                        is_hovered,
                                        config,
                                        theme,
                                    ));
                                }
                            }
                        }

                        if is_message_visible {
                            notify_visibility(
                                column,
                                2000.0,
                                notify_visibility::When::NotVisible,
                                Message::ExitingViewport(message.hash),
                            )
                        } else {
                            notify_visibility(
                                column,
                                1000.0,
                                notify_visibility::When::Visible,
                                Message::EnteringViewport(message.hash, urls),
                            )
                        }
                    } else {
                        element
                    }
                } else {
                    element
                };

                let content = if is_new_day && config.buffer.date_separators.show
                {
                    column![
                        row![
                            container(rule::horizontal(1))
                                .width(Length::Fill)
                                .padding(padding::right(6)),
                            text(
                                date.and_time(
                                    NaiveTime::from_hms_opt(0, 0, 0)
                                        .expect("midnight is valid")
                                )
                                .and_local_timezone(Local)
                                .single()
                                .map_or(
                                    // in the event of timezone weirdness,
                                    // revert to default format
                                    date.format(&DateSeparators::default().format),
                                    |datetime| {
                                        datetime.format(
                                            &config
                                                .buffer
                                                .date_separators
                                                .format,
                                        )
                                    }
                                )
                                .to_string()
                            )
                            .size(divider_font_size)
                            .style(theme::text::secondary)
                            .font_maybe(
                                theme::font_style::secondary(theme)
                                    .map(font::get)
                            ),
                            container(rule::horizontal(1))
                                .width(Length::Fill)
                                .padding(padding::left(6))
                        ]
                        .padding(2)
                        .align_y(iced::Alignment::Center),
                        content
                    ]
                    .into()
                } else {
                    content
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
            .get(&m.hash)
            .copied()
            .map_or(row_height, |h| h + line_spacing as f32)
    };

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

    let show_backlog_divier = if old.is_empty() {
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

    let divider = if show_backlog_divier {
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
                    .style(theme::text::secondary)
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
    height_cache: HashMap<message::Hash, f32>,
    pending_scroll_to: Option<keyed::Key>,
    is_scrolling_to: bool,
    visible_url_messages: HashMap<message::Hash, Vec<url::Url>>,
    pending_preview_exits: HashSet<message::Hash>,
    hovered_preview: Option<(message::Hash, usize)>,
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
            is_scrolling_to: false,
            visible_url_messages: HashMap::new(),
            pending_preview_exits: HashSet::new(),
            hovered_preview: None,
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        infinite_scroll: bool,
        kind: Kind,
        buffer: Option<&buffer::Upstream>,
        history: &mut history::Manager,
        clients: &mut client::Map,
        config: &Config,
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
                                &config.buffer,
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
                    Some(Event::ContextMenu(context_menu::update(message))),
                );
            }
            Message::Link(message::Link::Channel(server, channel)) => {
                return (
                    Task::none(),
                    Some(Event::OpenBuffer(
                        server,
                        Target::Channel(channel),
                        config.actions.buffer.click_channel_name,
                    )),
                );
            }
            Message::Link(message::Link::Url(url)) => {
                return (Task::none(), Some(Event::OpenUrl(url)));
            }
            Message::Link(message::Link::User(server, user)) => {
                let event = match config.buffer.nickname.click {
                    data::config::buffer::NicknameClickAction::OpenQuery => {
                        let query = target::Query::from(user);

                        Event::OpenBuffer(
                            server,
                            Target::Query(query),
                            config.actions.buffer.click_username,
                        )
                    }
                    data::config::buffer::NicknameClickAction::InsertNickname => {
                        Event::ContextMenu(context_menu::Event::InsertNickname(
                            user.nickname().to_owned(),
                        ))
                    }
                };

                return (Task::none(), Some(event));
            }
            Message::Link(message::Link::GoToMessage(
                server,
                channel,
                message,
            )) => {
                return (
                    Task::none(),
                    Some(Event::GoToMessage(server, channel, message)),
                );
            }
            Message::ScrollTo(keyed::Hit {
                key: _,
                hit_bounds,
                scrollable,
            }) => {
                self.is_scrolling_to = false;

                let max_offset = scrollable.max_vertical_offset();

                let top_inset = theme::resolve_line_height(&config.font) * 0.5;

                let offset = (hit_bounds.y - scrollable.content.y - top_inset)
                    .max(0.0)
                    .min(max_offset);

                if (offset - max_offset).abs() <= f32::EPSILON {
                    self.status = Status::Bottom;

                    if !matches!(self.limit, Limit::Bottom(_)) {
                        self.limit = Limit::Bottom(step_messages(
                            2.0 * self.pane_size.height,
                            config,
                        ));
                    }

                    return (
                        correct_viewport::scroll_to(
                            self.scrollable.clone(),
                            scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
                        ),
                        None,
                    );
                } else {
                    self.status = Status::Unlocked;

                    return (
                        correct_viewport::scroll_to(
                            self.scrollable.clone(),
                            scrollable::AbsoluteOffset { x: 0.0, y: offset },
                        ),
                        None,
                    );
                }
            }
            Message::Link(message::Link::ExpandCondensedMessage(
                server_time,
                hash,
            )) => {
                return (
                    Task::none(),
                    Some(Event::ExpandCondensedMessage(server_time, hash)),
                );
            }
            Message::Link(message::Link::ContractCondensedMessage(
                server_time,
                hash,
            )) => {
                return (
                    Task::none(),
                    Some(Event::ContractCondensedMessage(server_time, hash)),
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
            Message::ImagePreview(path, url) => {
                return (Task::none(), Some(Event::ImagePreview(path, url)));
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
            Message::HeightsCollected(heights) => {
                for (hash, height) in &heights {
                    self.height_cache.insert(*hash, *height);
                }

                let mut preview_changed = false;

                if !self.pending_preview_exits.is_empty() {
                    let rendered_hashes = heights
                        .iter()
                        .map(|(hash, _)| *hash)
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
        }
        (Task::none(), None)
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
    ) -> Task<Message> {
        let Some(history::View {
            old_messages,
            new_messages,
            ..
        }) = history.get_messages(&kind.into(), None, &config.buffer)
        else {
            // We're still loading history, which will trigger scroll_to_backlog
            // after loading. If this is set, we will scroll_to_message
            self.pending_scroll_to = Some(keyed::Key::Message(message));

            return Task::none();
        };

        let Some(target) = old_messages
            .iter()
            .chain(&new_messages)
            .find(|m| m.hash == message)
        else {
            return Task::none();
        };

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
        }) = history.get_messages(&kind.into(), None, &config.buffer)
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
        }) = history.get_messages(&kind.into(), None, &config.buffer)
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
        self.visible_url_messages.values().flatten()
    }
}

fn send_reaction(
    clients: &mut client::Map,
    buffer: Option<&buffer::Upstream>,
    history: &mut history::Manager,
    msgid: message::Id,
    text: String,
    unreact: bool,
) -> Option<()> {
    let buffer: &buffer::Upstream = buffer?;

    let target = buffer.target()?;
    let command = if unreact {
        Irc::Unreact {
            target: target.to_string(),
            msgid: msgid.clone(),
            text: text.clone(),
        }
    } else {
        Irc::React {
            target: target.to_string(),
            msgid: msgid.clone(),
            text: text.clone(),
        }
    };
    let encoded = message::Encoded::try_from(command).ok()?;
    clients.send(buffer, encoded, TokenPriority::User);
    let nick = clients.nickname(buffer.server())?;
    let reaction = reaction::Context {
        inner: Reaction {
            sender: nick.to_owned(),
            text,
            unreact,
        },
        target,
        in_reply_to: msgid,
    };
    let supports_echoes = clients.get_server_supports_echoes(buffer.server());
    if !supports_echoes {
        history.record_reaction(buffer.server(), reaction);
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

mod keyed {
    use data::message;
    use iced::advanced::widget::{self, Operation};
    use iced::widget::scrollable::{self, AbsoluteOffset};
    use iced::{Rectangle, Task, Vector, advanced};

    use crate::widget::{Element, Renderer, decorate};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        heights: Vec<(message::Hash, f32)>,
    }

    impl Operation<Vec<(message::Hash, f32)>> for CollectHeights {
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
            operate: &mut dyn FnMut(
                &mut dyn Operation<Vec<(message::Hash, f32)>>,
            ),
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
                && let Some(Key::Message(hash)) = state.downcast_ref::<Key>()
            {
                self.heights.push((*hash, bounds.height));
            }
        }

        fn finish(
            &self,
        ) -> widget::operation::Outcome<Vec<(message::Hash, f32)>> {
            if self.heights.is_empty() {
                widget::operation::Outcome::None
            } else {
                widget::operation::Outcome::Some(self.heights.clone())
            }
        }
    }

    pub fn collect_heights(
        scrollable: widget::Id,
    ) -> Task<Vec<(message::Hash, f32)>> {
        widget::operate(CollectHeights {
            active: false,
            scrollable_id: scrollable,
            heights: vec![],
        })
    }
}

fn preview_row<'a>(
    message: &'a data::Message,
    preview: &'a Preview,
    url: &url::Url,
    idx: usize,
    right_aligned_width: Option<f32>,
    max_prefix_width: Option<f32>,
    is_hovered: bool,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let content = match preview {
        data::Preview::Card(preview::Card {
            image: preview::Image { path, .. },
            title,
            description,
            ..
        }) => keyed(
            keyed::Key::Preview(message.hash, idx),
            button(
                container(
                    column![
                        text(title)
                            .shaping(text::Shaping::Advanced)
                            .style(theme::text::primary)
                            .font_maybe(
                                theme::font_style::primary(theme)
                                    .map(font::get)
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
                            .max_height(
                                config.preview.card.description_max_height,
                            )
                        }),
                        config.preview.card.show_image.then_some(
                            container(
                                image(path)
                                    .border_radius(
                                        if config
                                            .preview
                                            .card
                                            .round_image_corners
                                        {
                                            4
                                        } else {
                                            0
                                        }
                                    )
                                    .content_fit(ContentFit::ScaleDown)
                            )
                            .padding(Padding::default().top(8))
                            .max_height(config.preview.card.image_max_height)
                        ),
                    ]
                    .spacing(8)
                    .max_width(config.preview.card.max_width),
                )
                .padding(8),
            )
            .style(theme::button::preview_card)
            .on_press(Message::Link(message::Link::Url(url.to_string()))),
        ),
        data::Preview::Image(preview::Image { path, url, .. }) => keyed(
            keyed::Key::Preview(message.hash, idx),
            button(
                container(
                    image(path)
                        .border_radius(if config.preview.image.round_corners {
                            4
                        } else {
                            0
                        })
                        .content_fit(ContentFit::ScaleDown),
                )
                .max_width(config.preview.image.max_width)
                .max_height(config.preview.image.max_height),
            )
            .on_press(match config.preview.image.action {
                data::config::preview::ImageAction::OpenUrl => {
                    Message::Link(message::Link::Url(url.to_string()))
                }
                data::config::preview::ImageAction::Preview => {
                    Message::ImagePreview(path.to_path_buf(), url.clone())
                }
            })
            .padding(0)
            .style(theme::button::bare),
        ),
    };

    let url_string = url.to_string();
    let content: Element<'a, Message> =
        crate::widget::context_menu::context_menu(
            crate::widget::context_menu::MouseButton::Right,
            crate::widget::context_menu::Anchor::Cursor,
            crate::widget::context_menu::ToggleBehavior::KeepOpen,
            content,
            context_menu::Entry::url_list(
                config
                    .preview
                    .is_enabled(url.as_str())
                    .then_some(message.hidden_urls.contains(url)),
            ),
            move |entry, length| {
                entry
                    .view(
                        Some(context_menu::Context::Url {
                            url: url_string.as_str(),
                            message: Some(message.hash),
                        }),
                        length,
                        config,
                        theme,
                    )
                    .map(Message::ContextMenu)
            },
        )
        .into();

    let timestamp_gap = config
        .buffer
        .format_timestamp(&message.server_time)
        .map(|timestamp| {
            selectable_text(" ".repeat(timestamp.chars().count()))
        });
    let space = selectable_text(" ");

    let aligned_content = match &config.buffer.nickname.alignment {
        data::buffer::Alignment::Left => {
            row![timestamp_gap, space, content].into()
        }
        data::buffer::Alignment::Right => {
            let prefixes = message.target.prefixes().map_or(
                right_aligned_width.and_then(|_| {
                    max_prefix_width
                        .map(|width| selectable_text("").width(width))
                }),
                |prefixes| {
                    let text = selectable_text(
                        " ".repeat(
                            config
                                .buffer
                                .status_message_prefix
                                .brackets
                                .format(String::from_iter(prefixes))
                                .chars()
                                .count()
                                + 1,
                        ),
                    );

                    if let Some(width) = max_prefix_width {
                        Some(text.width(width))
                    } else {
                        Some(text)
                    }
                },
            );

            let with_access_levels = config.buffer.nickname.show_access_levels;
            let truncate = config.buffer.nickname.truncate;

            let nick = if let message::Source::User(user) =
                message.target.source()
            {
                let mut nick = selectable_text(
                    " ".repeat(
                        config
                            .buffer
                            .nickname
                            .brackets
                            .format(user.display(with_access_levels, truncate))
                            .chars()
                            .count(),
                    ),
                );

                if let Some(width) = right_aligned_width {
                    nick = nick.width(width);
                }

                Some(nick)
            } else {
                None
            };

            let timestamp_nickname_row =
                row![timestamp_gap, space, prefixes, nick];

            let space = selectable_text(" ");

            row![timestamp_nickname_row, space, content].into()
        }
        data::buffer::Alignment::Top => content,
    };

    let hide_button = if is_hovered {
        container(tooltip(
            button(center(icon::cancel()))
                .padding(5)
                .width(HIDE_BUTTON_WIDTH)
                .height(HIDE_BUTTON_WIDTH)
                .on_press(Message::HidePreview(message.hash, url.clone()))
                .style(|theme, status| {
                    theme::button::secondary(theme, status, false)
                }),
            config.tooltips.then_some("Hide Preview"),
            tooltip::Position::Top,
            theme,
        ))
    } else {
        container(space::horizontal().width(Length::Fixed(HIDE_BUTTON_WIDTH)))
    };

    // Iced hack: using a stack with right-aligned hide_button ensures the button always stays visible
    // at the edge of the content, even when the parent container is resized to a smaller width.
    let stack = stack![
        container(aligned_content)
            .padding(Padding::default().right(HIDE_BUTTON_WIDTH + 2.0)),
        right(hide_button),
    ];

    let content = container(stack)
        .align_y(alignment::Vertical::Top)
        .width(Length::Fill)
        .padding(Padding::default().top(4).bottom(4));

    mouse_area(content)
        .on_enter(Message::PreviewHovered(message.hash, idx))
        .on_exit(Message::PreviewUnhovered(message.hash, idx))
        .into()
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
