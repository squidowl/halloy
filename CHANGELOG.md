# Unreleased

Added:

- New configuration options
  - Right aligning nicks in buffers. See [configuration](https://halloy.squidowl.org/configuration/buffer.html#buffernickname-section).
  - Right aligning nicks in nicklist. See [configuration](https://halloy.squidowl.org/configuration/buffer).
  - Hiding `chghost` messages. See [configuration](https://halloy.squidowl.org/configuration/buffer.html#bufferserver_messages-section).
  - Overwrite nicklist `width` in channels. See [configuration](https://halloy.squidowl.org/configuration/buffer.html#bufferchannelnicklist-section).
  - Show/hide user access levels in buffer and nicklist. See [configuration](https://halloy.squidowl.org/configuration/buffer.html#bufferchannelnicklist-section)
  - `buffer_focused_action` added to `sidebar` to enable actions a focused buffer. See [configuration](https://halloy.squidowl.org/configuration/sidebar.html#sidebar-section).

Fixed:

- Expanded recognized login notifications (used to join channels that report themselves as requiring registration after logging in)
- Messages with multiple targets are correctly recorded into multiple buffers (and/or multiple times into the same buffer) client-side.
- Messages sent with a STATUSMSG prefix are recorded and indicated in the corresponding channel.
- Ability to position the sidebar at the top, bottom, right, or left. See [Sidebar configuration](https://halloy.squidowl.org/configuration/sidebar.html).
- 

Changed:

- Reworked themes to add better customization possibilities
  - **NOTE** Old theme files are not compatibile with the new format. However all the themes in the [theme community](https://halloy.squidowl.org/configuration/themes/community.html) has been updated to the new format.
- Unread indicator has changed from a boolean value to a enum. See [Sidebar configuration](https://halloy.squidowl.org/configuration/sidebar.html).
- Renamed `sidebar.default_action` to `sidebar.buffer_action`.


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
- Connection timeout when UI is suspended on an offscreen workspace due to channel backpressure
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

**BREAKING** Configuration file format has switched from `YAML` to `TOML`. Please vist the migration guide here: [halloy.squidowl.org/guides/migrating-from-yaml](https://halloy.squidowl.org/guides/migrating-from-yaml.html).

Added:

- Added command bar entry to open wiki website.

Changed:

- Configuration file now uses TOML instead of YAML
  - Renamed `[keys]` section to `[keyboard]`
  - Renamed `[buffer.channel.users]` section to `[buffer.channel.nicklist]`
  - Renamed `[buffer.input_visibility]` section to `[buffer.text_input]`
  - Removed `[dashboard]` section
    - Renamed `[dashboard.sidebar]` section to `[sidebar]`
  - Changed `exclude` from `[buffer.server_messages]` to two seperate settings
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
- New configuration option `dashboard.sidebar_default_action` to control pane behaviour when selecting buffers
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

- The last word of a message sometimes dissapeared
- Persist partial text input content when switching away from buffer
- Correctly load image on welcome screen

# 2023.1-alpha1 (2023-06-30)

Added:

- First release 🎉
