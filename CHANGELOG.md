# Unreleased

Added:

- Exit application from user menu
- Border radius on both image previews and cards displaying images through meta tags (configurable)
- Context menu item added to server-wide buffers to close all queries
- Messages that fail to send are dimmed and marked, and can be deleted or re-sent (for servers that support the `echo-message` capability)
- Reload Halloy by sending `SIGUSR1` on Unix system
- Ability to hide the indicator when a new version of Halloy is available
- Fuzzy matching when searching in command bar
- Setting to disable file transfers (DCC), removing it from the UI
- Ability to hide nickname for consecutive messages from the same user
- Ability to specify a distinct sound for each match highlight set
- Setting to notify for every message in a channel
- Per server custom proxy configuration option
- Ability to exclude previews from specific server messages (e.g. topic, part, quit)
- Ability to expand & contract condensed messages via left-click
- Change-host and kick server messages messages can be condensed
- `/plain` command to send text with automatic formatting disabled
- `Whowas` instead of `Whois` action in context menu for offline nicknames

Fixed:

- Larger fonts (and font sizes) can be used without blanking out the input box
- Standard Exif rotation data is used when displaying images
- Server messages reporting an inability to send to a nick/channel are routed to the relevant buffer
- Hide jitter in backlog divider caused by read marker updates when marking read on send
- Flickering logo on Welcome screen
- Channels with the same name as servers now appear before servers when searching in command bar

Changed:

- By default the backlog separator is hidden when all messages in a pane have been marked as read (`buffer.backlog_separator.hide_when_all_read`)
- Buffer context menu items in sidebar are sorted alphabetically
- Unicode nicknames are now supported
- User menu configuration moved to `[sidebar.user_menu]` section

Thanks:

- Contributions: @kasperkronborg, @mistivia, @oooo-ps
- Bug reports: @Frikilinux, ThePendulum, @crabbedhaloablution, @findus, @Darksecond
- Feature requests: @cyrneko, ilya, abby, @clorteau, @RobertCraigie, @dei-layborer, @classabbyamp, @4e554c4c

# 2025.11 (2025-10-27)

Fixed:

- Regression when loading chat history in a channel

Thanks:

- Bug reports: @Erroneuz

# 2025.10 (2025-10-27)

Added:

- `/sysinfo` command to send system information to current buffer
- Copy URLs from context menu by right-clicking on links
- Settings to condense multiple consecutive server messages into a single abbreviated message
- `/detach` command to detach from channels (soju feature)
- Support for USERINFO CTCP query
- Configurable anti-flood protection for servers that do not advertise SAFERATE
- Channel modes will be retrieved on joining a channel (to be shown in the pane title bar)
- Display own nickname next to text input field
- Server messages can be dimmed (enabled by default), to lessen their visual impact relative to user messages
- Context menu to timestamps that shows the message date, and will copy the timestamp to the clipboard when clicked
- Setting to open new buffers scrolled to the bottom (rather than scrolled to the last unread message, the default and current behavior)
- macOS: Ability to adjust content rendering to account for titlebar
- Setting to remove all window decorations
- New sounds: Zone, Bloop, Drop, Tweep.

Fixed:

- Enforce delay between notifications
- Larger fonts (and font sizes) can be used without blanking out the sidebar or pane title bar(s)
- The primary clipboard (with copy on selection & paste with middle click) is supported on Linux
- Sound effects are now properly reloaded when config file is refreshed from within the application
- Filters are applied only to the server they are specified for
- Filters are renormalized properly when ISUPPORT is updated
- Crash when typing `/bouncer` into the message box
- Scroll state no longer changes rapidly when long previews for URL(s) are present in history
- Sidebar menu always takes one click to open (would sometimes require two clicks)

Changed:

- The logs pane will only contain log messages since the last of launch of Halloy (i.e. for the current session only)
- Always show version number in user menu

Thanks:

- Contributions: @Toby222, @Frikilinux, @4e554c4c, @apollo7321, @Bartkk0, @seraxis
- Bug reports: @Toby222, @deepspaceaxolotl, @zhelezov, @Erroneuz, @dgz0, @csmith, @otonoton, @mefistoso
- Feature requests: @j0lol, @xcfrg, ThePendulum, i_like_halloy

# 2025.9 (2025-09-16)

Added:

- Added different unread icons for highlights vs regular unread messages
- Setting to change icons for unread messages and unread highlight messages
- Setting to change server icon size
- Setting to control spacing between messages in buffers
- Ability to specific multiple themes which then gets picked randomly at launch
- Notification indicator on sidebar menu for warning & error log messages
- Ability to mark as read on buffer close only if scrolled to the bottom of the buffer (new default behavior)
- `/part`, `/topic`, `/mode`, `/kick`, and `/ctcp` commands can have their target(s) argument skipped when it can be inferred from context (e.g. `/topic` will target the current channel by default when used in a channel buffer)
- `/cleartopic` command to remove a channel's topic (will target the current channel by default when used in a channel buffer)
- Ability to hide and theme `kick` server messages
- Setting to control whether or not to restore on launch the panes that were open when Halloy last closed
- Setting for split axis chosen as the shorter dimension of the focused pane (new default behavior)
- Setting for split axis chosen as the shorter dimension of the largest splittable pane
- `soju.im/bouncer-networks` support
- Setting to hide the backlog divider when all messages in the buffer have been read
- Setting to specify whether nickname highlighting is case sensitive
- Setting to hide topic changes

Fixed:

- Receiving files over DCC will no longer stall when sender stops accepting received receipts
- Attempting to navigate to the next/previous unread buffer, when there is no such buffer, will no longer clear the buffer
- Window position is now validated, preventing windows from opening on disconnected monitors
- When kicked from a channel the kick message will be broadcast in the server buffer (which remains open) as well as in the channel history (which is closed on kick)
- Preview images with large dimensions will not be displayed if larger than the allowed buffer size
- Do not activate the mark as read buffer when blocked/hidden messages are unread in the buffer
- Nickname highlighting is case insensitive by default (and uses the server's specified casemapping)
- Nickname changes properly broadcast in all channels user is in
- Page up/down behavior moves by consistent page length, and will no longer get stuck at the top of a buffer
- `file_transfer.save_directory` is now affected by path substitutions (tilde expansion & relative path prefixing)

Changed:

- Relative paths in configuration files are now prefixed with config dir

Thanks:

- Contributions: @Death916, @4e554c4c, @freakyy85, @hashcatHitman, @5GameMaker
- Bug reports: @privacyadmin, @rlndd, @wingedonezero, @Seishiin, @Erroneuz, @andar1an, freakyy85, ThinkT510, alexia, @darienm, quaff, @zhelezov
- Feature requests: @deepspaceaxolotl, @4e554c4c, @seraxis, RebeLLz, @cvengler, @barretgoat, @remexre, @dmd

# 2025.8 (2025-07-31)

Added:

- Clear buffers with `/clear` command
- Notification indicator on sidebar menu for application updates and file transfers

Fixed:

- Nicknames in query was always shown as offline
- macOS now properly prioritizes XDG config directory (`~/.config/halloy`) over user config directory (`~/Library/Application Support/halloy`)

# 2025.7 (2025-07-26)

Added:

- Ability to ignore nicknames on server level or on a per-channel basis
- Ability to truncate nicknames in buffer
- New option to customize the appearance of offline nicknames
- Specify whether nicknames in buffers appear with the user's current status or status at time of sending the message
- Log level filtering for the log buffer (<kbd>ctrl</kbd> + <kbd>l</kbd>) and log file
- Per toast type setting for showing content in toasts
- Ability to hide `nick` and `mode` change server messages
- Context menu item added to server-wide buffers to mark all messages as read
- Auto-accept file transfers, with support for nick and mask filtering
- Adjustable scrollbar sizing

Fixed:

- Sidebar menu is now drawn with the correct offset
- Prevent messages with previews from shifting position on hover
- Recognize registration completion on older servers without capability negotiation (i.e. send `on_connect`, `umodes`, etc.)

Changed:

- Optimized message loading by implementing lazy loading: buffers now load only enough messages to fill the visible area instead of always loading 500+ messages.
- The `[buffer.away]` section has been split into `[buffer.channel.nicklist]` and `[buffer.nickname]` for greater flexibility
- Colorize the level text in log buffer based on theme
- Scrollbars now use a dedicated `scrollbar` color instead of `horizontal_rule`
- Tor is now an optional feature. Build with `--features tor` to enable Tor proxy support
- Topic "set at" time is now shown with local timezone offset, instead of UTC's

Thanks:

- Contributions: @4e554c4c, @KE0VVT, @englut, @KaiKorla, @root-goblin
- Bug reports: @darienm, @mercster, @ChrisRenfrow, @glguy
- Feature requests: @rossburton, death916, @Celeo, @TehPeGaSuS, @gdluca, @darienm, @RealKindOne, @Tahomaru, @tribly

# 2025.6 (2025-06-14)

Added:

- New application icon
- Show channel mode in panel if available
- Configuration options for font weight and bold weight
- `/delay <seconds>` command that can be used to control the timing of commands sent on connect
- Configuration option for the ordering of servers in the sidebar

Fixed:

- Handling of RPL_NOWAWAY & RPL_UNAWAY to reflect user's own AWAY state
- Draft messages are saved while navigating message history (i.e. pressing ↑ then ↓ in the input box will restore an unsent message after displaying sent message history)

Thanks:

- Contributions: @englut, @KaiKorla, @kit-ty-kate, @mhegreberg, @ohhskar, Rune Seir
- Bug reports: @awbradle
- Feature requests: @darienm, @kasper93, @auronandace, @n-kalosha

# 2025.5 (2025-05-05)

Added:

- Ability to show a modal prompt before opening a URL
- WHOIS command now accepts optional server parameter
- CTCP requests through commands and user context
- Title bar button and keyboard shortcut to mark a buffer as read (will update the read marker as well, if the `read-marker` capability is available)
- Mark as Read settings to control when buffers are automatically marked as read
- `/hop` command. `/hop` parts the current channel and joins a new one
- Settings to limit passwords read from a file to the first line of the file only (on by default)
- Receive `WALLOPS` messages in the server buffer (color configurable in themes)

Changed:

- Clicking to insert a username will now use same suffixes specified for autocomplete
- Emoji picker will only show once there are two characters after `:` (by default, configurable)
- Autocomplete will match users based on how recently they were seen in the channel (by default, configurable)
- Include & exclude settings for previews apply to queries as well as channels

# 2025.4 (2025-04-07)

Added:

- All toast notifications now display the server name
- Server messages (join, part, etc.) are now user-aware and will color nicknames accordingly
- Actions will be colored with the same features as regular messages (nicknames, urls, etc)
- Shortcuts for cycling buffers with unread message(s)
  - Cycle to next buffer with unread message(s) <kbd>ctrl</kbd> + <kbd>`</kbd>
  - Cycle to previous buffer with unread message(s) <kbd>ctrl</kbd> + <kbd>~</kbd>
- Navigate commands & emoji picker with ↑ & ↓ (as well as Tab & Shift-Tab)
- Expanded command completion and error checking/reporting
- Various shortcuts for https://themes.halloy.chat/
  - The Command Bar now includes a "Discover More Themes" action, which opens the website
  - The Theme Editor now has a "Share Theme with Community" option that opens the submit page with the theme payload pre-filled
- Ability to adjust alpha value of away users
- New action settings section with settings to control how to open buffers in response to various user actions

Fixed:

- No longer freezes under Wayland when window(s) are not visible
- Emoji picker is no longer case sensitive (i.e. :thinking: and :THINKING: will both match 🤔)
- Full support for the `/notice` command
  - Message text will no longer be truncated to the first word
  - Sent notices appear in message history
  - `/notice <targets>` (without any message text) will open a pane for the target(s) without sending a message
- `/msg <targets>` supports more than one target when opening pane(s) without sending a message

Changed:

- Improved font rendering on Windows for better readability
- Sidebar buffer action settings have been moved to the new action settings section
  - `sidebar.buffer_action` → `actions.sidebar.buffer`
  - `sidebar.buffer_focused_action` → `actions.sidebar.focused_buffer`

# 2025.3 (2025-03-14)

Added:

- Highlights by regex and words
- Time remaining is now displayed during file transfers
- Sidebar now automatically adjusts its width based on content
- Ability to customize default pane splitting direction (vertical or horizontal)
- Clicking a nickname is now configurable. The default behavior has changed to open a query.
- Ability to quit Halloy through command bar and keyboard shortcut
- New shortcuts for scrolling a buffer:
  - Scroll buffer up a page <kbd>pageup</kbd> (<kbd>Fn</kbd> + <kbd>↑</kbd> on macOS)
  - Scroll buffer down a page <kbd>pagedown</kbd> (<kbd>Fn</kbd> + <kbd>↓</kbd> on macOS)
  - Scroll to top of buffer <kbd>ctrl</kbd> + <kbd>↑</kbd> (<kbd>⌘</kbd> + <kbd>↑</kbd> on macOS)
  - Scroll to bottom of buffer <kbd>ctrl</kbd> + <kbd>↓</kbd> (<kbd>⌘</kbd> + <kbd>↓</kbd> on macOS)

Changed:

- `[file_transfer.save_directory]` is now default download path for transfers. If set, files will be downloaded there by default. Otherwise, you'll be prompted to choose a location
- Ability to dynamically select dark or light theme based on OS appearance.
- `/msg <target>` (without any message text) will now open a pane for the target without sending a message
- Text input of the active pane will remain focused after all actions
- Focus last known active pane on launch
- Changed maximize / restore buffer shortcuts to <kbd>ctrl</kbd> + <kbd>shift</kbd> + <kbd>↑</kbd> / <kbd>ctrl</kbd> + <kbd>shift</kbd> + <kbd>↓</kbd> (<kbd>⌘</kbd> + <kbd>shift</kbd> + <kbd>↑</kbd> / <kbd>ctrl</kbd> + <kbd>shift</kbd> + <kbd>↓</kbd> on macOS) to prevent collision with new scroll shortcuts

# 2025.2 (2025-02-20)

Added:

- Emoji picker accessible via `:` in text input
- Automatically replace `:shortcode:` with corresponding emoji
- Added an option to show or hide images in preview cards by default

Fixed:

- [`infinite_scroll`](https://halloy.squidowl.org/configuration/buffer.html?highlight=infinite#infinite_scroll) was defaulting to `false`, contrary to its documented default value.  Now defaults to `true`
- A rare issue where the app opens smaller than when closed
- Closing the application while it is minimized (on Windows) causes it to open with the wrong size and position next time
- TLS connection issue on Windows (10+)

# 2025.1 (2025-02-02)

Added:

- Image and card previews for url links
- Ability to disable dimming of usernames when away
- Ability to customize autocomplete sorting direction and completion suffixes
- Ability to hide command descriptions
- Ability to set delay before triggering notifications
- Ability to toggle fullscreen
- Ability to exclude and include notifications.
- New IRCv3 extensions support
  - `chathistory`
  - `setname`
  - Standard Replies
- Case mapping support via `ISUPPORT`
- Restore last known window position and size at launch
- Date separators in buffers when day changes
- Show nickname in user-context menu
- Tor support as a proxy option

Fixed:

- Long username & password combinations could cause SASL authentication to fail
- `nick_password_command` is now working as intended
- Don't add suffix (`: `) to prompt if there is no valid nick to autocomplete
- Append `.toml` filename when saving themes on macOS
- Highlight notifications are correctly triggered for `/me` actions

Changed:

- Changed focus buffer shortcuts to include <kbd>ctrl</kbd> (<kbd>⌘</kbd> on macOS) to avoid interfering with default text input shortcuts for word navigation (<kbd>⌘</kbd> + <kbd>←</kbd>, <kbd>⌘</kbd> + <kbd>→</kbd>)

# 2024.14 (2024-10-29)

Fixed:
  - CPU spiking due to memory leak in upstream crate.

# 2024.13 (2024-10-27)

Added:
- Highlight buffer to view highlights across all servers. Can be opened via sidebar button or command bar.
- Logging buffer to view logs. Can be opened via sidebar button or command bar.
- New hotkeys
  - Toggle channel topic (<kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>t</kbd> (macOS: <kbd>⌘</kbd> + <kbd>⌥</kbd> + <kbd>t</kbd>))
  - Toggle file_transfers buffer (<kbd>Ctrl</kbd> + <kbd>j</kbd> (macOS: <kbd>⌘</kbd> + <kbd>j</kbd>))
  - Toggle logs buffer (<kbd>Ctrl</kbd> + <kbd>l</kbd> (macOS: <kbd>⌘</kbd> + <kbd>l</kbd>))
  - Toggle theme editor window (<kbd>Ctrl</kbd> + <kbd>t</kbd> (macOS: <kbd>⌘</kbd> + <kbd>t</kbd>))
- New configuration options
  - Dynamically select dark or light theme based on OS appearance. See [configuration](https://halloy.squidowl.org/configuration/themes/index.html).
  - Ability to define a shell command for loading a NICKSERV password. See [configuration](https://halloy.squidowl.org/configuration/servers/index.html#nick_password_command)
  - Ability to define a shell command for loading a SASL password. See [configuration](https://halloy.squidowl.org/configuration/servers/sasl/plain.html)
  - Show/hide sidebar button to view logs. See [configuration](https://halloy.squidowl.org/configuration/sidebar/buttons.html#logs)
  - Ability to align nicknames above messages. See [configuration](https://halloy.squidowl.org/configuration/buffer/nickname.html#alignment).

Fixed:

- Inverted scrolling direction.
- Only highlight if user nick isn't part of another word.
- Errors from password commands are now caught and displayed to the user.

Removed:

- Configuration options:
  - `[sidebar.buttons]` in favor for the new User Menu

# 2024.12 (2024-09-17)

Added:

- Multi-window support: Pop out panes into separate windows.
- In-app theme editor with ability to with share it via a halloy:// URL.
- Clickable users and channels in buffers.
- New configuration options
  - Ability to include or exclude channels for server messages (join, part, etc.). See [configuration](https://halloy.squidowl.org/configuration/buffer/server_messages/index.html).
  - Ability to color nicknames within channel messages. See [configuration](https://halloy.squidowl.org/configuration/buffer/channel/message.html#nickname_color)
  - Ability to define a shell command for loading a server password. See [configuration](https://halloy.squidowl.org/configuration/servers/index.html#password_command)
- Enable support for IRCv3 `msgid` and `read-marker`

Fixed:

- Regression where messages sent to ZNC virtual users were routed to the wrong buffer (e.g. `*status` routed to `status`).
- Timeouts that could occur while Halloy was running in the background.

Changed:

- Default Ghost sequence changed to `REGAIN`.

# 2024.11 (2024-09-04)

Added:

- New configuration options
  - Right aligning nicks in buffers. See [configuration](https://halloy.squidowl.org/configuration/buffer/nickname.html#alignment).
  - Right aligning nicks in nicklist. See [configuration](https://halloy.squidowl.org/configuration/buffer/channel/nicklist.html#alignment).
  - Hiding `chghost` messages. See [configuration](https://halloy.squidowl.org/configuration/buffer/server_messages/change_host.html).
  - Overwrite nicklist `width` in channels. See [configuration](https://halloy.squidowl.org/configuration/buffer/channel/nicklist.html#width).
  - Show/hide user access levels in buffer and nicklist. See [configuration](https://halloy.squidowl.org/configuration/buffer/channel/nicklist.html#show_access_levels)
  - `buffer_focused_action` added to `sidebar` to enable actions a focused buffer. See [configuration](https://halloy.squidowl.org/configuration/sidebar/index.html#buffer_focused_action).
  - Notification for direct messages. See [notification configuration](https://halloy.squidowl.org/configuration/notifications.html).
- Enable support for IRCv3 [`Monitor`](https://ircv3.net/specs/extensions/monitor) and [`extended-monitor`](https://ircv3.net/specs/extensions/extended-monitor)

Fixed:

- Expanded recognized login notifications (used to join channels that report themselves as requiring registration after logging in)
- Messages with multiple targets are correctly recorded into multiple buffers (and/or multiple times into the same buffer) client-side.
- Messages sent with a STATUSMSG prefix are recorded and indicated in the corresponding channel.
- Ability to position the sidebar at the top, bottom, right, or left. See [configuration](https://halloy.squidowl.org/configuration/sidebar/index.html#position).
- `/mode` requires too many parameters for certain commands.

Changed:

- Reworked themes to add better customization possibilities
  - **NOTE** Old theme files are not compatible with the new format. However all the themes in the [theme community](https://halloy.squidowl.org/configuration/themes/community.html) has been updated to the new format.
- Unread indicator has changed from a boolean value to a enum. See [configuration](https://halloy.squidowl.org/configuration/sidebar/index.html#unread_indicators).
- Renamed `sidebar.default_action` to `sidebar.buffer_action`.
- Auto-completing (with tab) a nickname at the beginning of the input line will append ': ' (colon space). Otherwise, a space is appended to the completion.

Removed:

- Removed `hex` configuration option for server messages and nicknames.
  - `hex` was previously used to overwrite a color value. This is now done through the new theme format.



# 2024.10 (2024-08-04)

Added:

- Small icon in sidemenu when a new release is available
- Enable support for IRCv3 `chghost`, `account-notify`, and `extended-join`

Removed:

- Persistent window position and size due to an upstream bug

# 2024.9 (2024-07-29)

Added:

- Rich formatted and clickable URLs
- Text formatting through `/format` command. For more details, see [text formatting guide](https://halloy.squidowl.org/guides/text-formatting.html).
- Support for CTCP queries CLIENTINFO, PING, SOURCE, and VERSION
- Custom notification sounds. Use your own sounds or select from a few new built-in options. For more details, see [notification configuration](https://halloy.squidowl.org/configuration/notifications.html).
- Support DCC Send requests with spaces in the filename
- Reload config button in Sidebar

Fixed:

- Text input missing key presses in certain instances
- Connection timeout when UI is suspended on an offscreen workspace due to channel back pressure
- Raw commands are passed through unmodified
- AWAY command cuts off the away message

# 2024.8 (2024-07-05)

Added:

- Ability to open `irc://` and `ircs://` URL schemes
- Ability to overwrite nickname colors by providing a hex string (see [buffer configuration](https://halloy.squidowl.org/configuration/buffer.html#buffernicknamecolor-section)).
- Ability to overwrite server & internal message colors by providing a hex string (see [buffer configuration](https://halloy.squidowl.org/configuration/buffer.html#bufferserver_messages-section)).
- Configurable shortcuts for "Leave Buffer" and "Toggle Sidebar" actions (see [keyboard shortcuts configuration](https://halloy.squidowl.org/configuration/keyboard.html)).
- Ability to remember window position and size when reopened.
- Ability to hide unread indicators in sidebar (see [sidemenu configuration](https://halloy.squidowl.org/configuration/sidebar.html))

Fixed:

- UTF-8 channel name rendering in sidebar and in pane title bars.

# 2024.7 (2024-05-05)

Added:

- Reload configuration file from within the application (<kbd>Ctrl</kbd> + <kbd>r</kbd> (macOS: <kbd>⌘</kbd> + <kbd>r</kbd>))
- Allow configuration of internal messages in buffer (see [buffer configuration](https://halloy.squidowl.org/configuration/buffer.html#bufferinternal_messages-section))
- User information added to context menu
- Support for IRCv3 `CAP NEW` and `CAP DEL` subcommands
- Enable support for IRCv3 `multi-prefix`, `message-tags`, `WHOX`, and `UTF8ONLY`
- Dynamic commands and tooltips added to command auto-completion via `ISUPPORT`
- Added support for `socks5` proxy configuration (see [proxy configuration](https://halloy.squidowl.org/configuration/proxy.html))
- Added support for `http` proxy configuration (see [proxy configuration](https://halloy.squidowl.org/configuration/proxy.html))

Changed:

- Simplified onboarding experience for users without a `config.toml` file
- MacOS will now also look in `$HOME/.config/halloy` for `config.toml`.
- Context menus can be dismissed by pressing Escape
- Join channels that report themselves as requiring registration after logging in

Fixed:

- No longer automatically reconnects to a server after using the `/QUIT` command.

# 2024.6 (2024-04-05)

Added:

- File transfer support (see [file transfer configuration](https://halloy.squidowl.org/configuration/file_transfer.html))
- Adjust WHO polling for servers without `away-notify` (see [server configuration](https://halloy.squidowl.org/configuration/servers.html))
- Tooltips on application buttons (see [tooltips configuration](https://halloy.squidowl.org/configuration/tooltips.html)).
- Sidebar menu buttons (see [sidebar configuration](https://halloy.squidowl.org/configuration/sidebar.html#sidebarbuttons-section)).
- Display current version, and latest remote version in command bar
- Allow reading passwords from files in server configuration
- Allow configuration to specify a server's NickServ IDENTIFY command syntax

Fixed:

- Accept '@' in usernames to support bouncers that use the user@identifier/network convention
- Prevent rare scenario where broadcast messages' timestamp would not match time the messages are received
- Fix SASL on macos by using RUSTLS backend

Changed:

- MacOS icon to better follow Apple's [Human Interface Guidelines](https://developer.apple.com/design/human-interface-guidelines/app-icons)

# 2024.5 (2024-03-21)

**BREAKING** Configuration file format has switched from `YAML` to `TOML`. Please visit the migration guide here: [halloy.squidowl.org/guides/migrating-from-yaml](https://halloy.squidowl.org/guides/migrating-from-yaml.html).

Added:

- Added command bar entry to open wiki website.

Changed:

- Configuration file now uses TOML instead of YAML
  - Renamed `[keys]` section to `[keyboard]`
  - Renamed `[buffer.channel.users]` section to `[buffer.channel.nicklist]`
  - Renamed `[buffer.input_visibility]` section to `[buffer.text_input]`
  - Removed `[dashboard]` section
    - Renamed `[dashboard.sidebar]` section to `[sidebar]`
  - Changed `exclude` from `[buffer.server_messages]` to two separate settings
    - `enabled = bool`
    - `smart = number`
- Use primary text color instead of accent color for `solid` nicknames
- Op and Voice context menu items hidden in channels where the user is not an Op

Fixed:

- `WHOIS` is now correctly formatted when printed in buffers.

# 2024.4 (2024-03-15)

Added:

- Configuration option to enable a topic banner in channels. This can be enabled under `buffer.channel.topic`
- Messages typed into the input will persist until sent. Typed messages are saved when switching a pane to another buffer, then
  are restored when that buffer is returned to.
- Added display of highest access level in front of nicks in chat, mirroring the format in the nick list
- Added ability to toggle Op and Voice from user context menu in channels

Fix:

- Context menus now shows buttons as expected
- Buttons on help screen is now correctly sized
- Text input is now disabled when not connected to a channel
- When someone is kicked, it is now correctly shown
- Query messages sent by another client will now correctly be displayed
- Prevent flooding server by grouping channels together in as few JOIN messages as possible

Changed:

- Various UI changes
  - Ensured consistent padding in channel buffer
  - Unified styling for dividers

Security:

- `chrono` [RUSTSEC-2020-0071](https://rustsec.org/advisories/RUSTSEC-2020-0071)

# 2024.3, 2024.2 (2024-03-05)

Added:

- Option to colorize nicks in the nick list (and an option to control it separately from in the buffer)
- Option to control application scale factor

Fixed:

- Input not visible on Server and Query (DM) buffers
- Clipped buttons in context menu

Changed:

- Improved user experience in text input when auto-completing a nickname.
- Configuration option `server_messages` changed `exclude` from a boolean value to [`All`, `None` or `!Smart seconds`].
  - `All` excludes all messages for the specific server message.
  - `None` [default] excludes no messages for the specific server message.
  - `!Smart seconds` shows the server message if the user has sent a message in the given time interval (seconds) prior to the server message.

# 2024.1 (2024-02-07)

Added:

- Configuration option `servers.<name>.sasl.external.key` added to support loading a separate PEM PKCS #8 encoded key for SASL authentication.

Changed:

- Focus an available pane on launch, so that launch behavior follows typical use (e.g. if `dashboard.sidebar.default_action`
  is set to `replacePane`, then selecting a channel in the sidebar will replace the focused pane instead of opening a new pane)
- `hidden_server_messages` has been changed to `server_messages` and additional customization has been added:
  - Exclude messages [join, part, quit].
  - Adjust username format.

Fixed:

- Accept '\*' as a legal special symbol for usernames
- Accept '/' in usernames, ensuring correct parsing for bouncers using the nick/server convention
- Create the configuration directory correctly, if it does not exist yet.

# 2023.5 (2023-11-12)

Added:

- IRCv3 capability `userhost-in-names` support added
- IRCv3 capability `invite-notify` support added
- Configuration option `dashboard.sidebar.width` to control sidebar width.
- Configuration option `notification` to control and enable notifications

Changed:

- Limit messages to 512 bytes in length, including the trailing CR-LF characters.
- Configuration option `dashboard.sidebar_default_action` now moved to `dashboard.sidebar.default_action`

# 2023.4 (2023-08-03)

Added:

- Command bar (opened by pressing (<kbd>Ctrl</kbd> + <kbd>k</kbd> (macOS: <kbd>⌘</kbd> + <kbd>k</kbd>)))
- Configurable keyboard shortcuts for common actions, such as changing buffer focus, maximize / restoring buffer size,
  cycling channels in the buffer and more! A new `keys` section has been added to the config file, reference the
  [wiki](https://github.com/squidowl/halloy/wiki/Keyboard-shortcuts) for more details.
- Single clicking on a user will insert nickname to input
- Configuration option `on_connect` to execute commands once connected to a server, reference the
  [wiki](https://github.com/squidowl/halloy/wiki/Configuration#on-connect-commands) for more details.

Changed:

- Instead of using hostname as fallback, we now always derive the seed for unique user colors from their nickname

Fixed:

- Set the window application id on Linux to `org.squidowl.halloy`
- Correctly display all arguments when receiving MODE command

# 2023.3 (2023-07-27)

**Message history and dashboard state will be reset due to a breaking change. We've switched to a more flexible storage format
to ensure future breakages won't occur.**

Added:

- Away-notify extension added for supported servers
- SASL support for PLAIN & EXTERNAL. The following per-server config keys have been added:
  - PLAIN - `sasl.plain.username` & `sasl.plain.password`
  - EXTERNAL - `sasl.external.cert` is a path to the PEM encoded X509 cert
- Configuration option `buffer.hidden_server_messages` to hide server messages from the provided array of sources: ["join", "part", "quit"]
- Configuration option `buffer.input_visibility` to control input field visibility: always shown or following the focused buffer.
- Portable mode - if a config file exists in the same directory as the executable, all Halloy data will be saved to that directory
- Upon joining a channel, display the channel mode in the buffer
- When querying an away user, you will see an away message
- Autocomplete on joined channels

Changed:

- Away users will be appear slightly transparent in nicklist
- Configuration option `new_buffer` has been renamed to `buffer`. `new_buffer` key will still work for backwards compatibility.
- Migrated to our own internal IRC backend. This should allow for quicker development against extensions and bug fixes.

Fixed:

- Changes done in the config file are now properly applied to the old buffers
- Text and colors on light themes will no longer appear washed out
- All WHOIS responses are now properly routed to the buffer where the request was made (text input or via context menu)
- Accessing text input history will only populate the current buffer, not all of them
- Text from input box can be copied to clipboard
- Prevent text input cursor from blinking when window loses focus

# 2023.2 (2023-07-07)

Added:

- Nickname completions in text input with <kbd>Tab</kbd>
- Previously sent messages can be accessed per buffer in the text input with <kbd>↑</kbd> / <kbd>↓</kbd> arrows
- New configuration option `dashboard.sidebar_default_action` to control pane behavior when selecting buffers
- Messages from other users containing your nickname are now highlighted
- Themes directory where users can add their own theme files
- Broadcast nickname changes to relevant channels and queries.
- Broadcast quit messages to relevant channels and queries.
- Better error descriptions on connection failures
- Support RAW command
- Drag & drop buffers to the edges for better customization of the grid
- Whois messages are printed in the currently focused buffer

Changed:

- Default channel in `config.yaml` has been changed to `#halloy` (from `##rust`)
- `palette` field has been deprecated and replaced by `theme` in `config.yaml`
- Sorting channel nicknames
- Title headers has been changed to also display user count for channels
- Copy change: "Leave" has been changed to "Close query" for queries

Fixed:

- The last word of a message sometimes disappeared
- Persist partial text input content when switching away from buffer
- Correctly load image on welcome screen

# 2023.1-alpha1 (2023-06-30)

Added:

- First release 🎉
