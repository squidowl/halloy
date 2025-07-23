# Single pane

The settings below will configure Halloy to have a single pane (or fixed number of panes) in regular use.  When needed, new panes can be opened via the context menu on sidebar items (e.g. right-click on a channel in the sidebar and select "Open in new pane").

```toml
[actions.buffer]
click_channel_name = "replace-pane"
click_highlight = "replace-pane"
click_username = "replace-pane"
local = "replace-pane"
message_channel = "replace-pane"
message_user = "replace-pane"

[actions.sidebar]
buffer = "replace-pane"
```
