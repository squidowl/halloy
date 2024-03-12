# Unreleased

Added:

- Configuration option to enable a topic banner in channels. This can be enabled under `buffer.channel.topic`

Fix:

- Context menus now shows buttons as expected
- Buttons on help screen is now correctly sized

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

- Accept '*' as a legal special symbol for usernames
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
