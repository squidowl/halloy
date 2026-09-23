# Internal Buffers

Internal buffers are buffers managed by Halloy instead of an IRC server.

You can open them from the user menu or command bar. You can also
[add them to the sidebar](/configuration/sidebar#internal-buffers).

- **Channel Monitor** shows messages from all joined channels in one place.
  [Configuration](/configuration/channel-monitor)
- **Highlights** shows messages that highlighted you.
  [Configuration](/configuration/highlights)
- **Logs** shows Halloy log messages.
  [Configuration](/configuration/logs)
- **File Transfers** shows your active and completed file transfers.
  [Configuration](/configuration/file-transfer)
- **Channel Discovery** shows channels available on a server.
  [Click action configuration](/configuration/actions#click_channel_discovery)
- **Config Editor** lets you edit Halloy's configuration inside Halloy.
- **Search** searches messages across all servers, channels and queries.
  See [Search](#search).

You can change how internal buffers open with the
[`open_internal` action](/configuration/actions#open_internal).

## Search

Open Search from the command bar, the user menu, the
[`search` key binding](/configuration/keyboard), a nickname's context menu
("Search messages"), or with `/search [text]`.

Search matches whole words, and the last word as you type it, so `rel` finds
"release". Results are newest first; click the channel name on a result to jump
to the message.

- `"exact phrase"` matches words next to each other.
- `from:nick` only shows messages from `nick`.
- `in:#channel` only shows messages in `#channel` (or in a query with a nick).

Only messages sent by users are searched. History from before Halloy stored it
in SQLite becomes searchable once its buffer has been opened.
