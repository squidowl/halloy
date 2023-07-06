# Unreleased

Added:

- New configuration option `dashboard.sidebar_default_action` allows controlling the pane behaviour when selecting channels in the sidebar
- Support for RAW command
- Messages from other users containing your nickname are now highlighted using the `info` colour
- Previously sent messages can be accessed per buffer in the text input with up / down arrows
- Themes directory where users can add their own theme files
- Nickname completions in text input with <kbd>Tab</kbd>

Changed:

- Default channel in `config.yaml` has been changed to `#halloy` (from `##rust`)
- `palette` field has been deprecated and replaced by `theme` in `config.yaml`
- Sorting channel nicknames
- Title headers has been changed to also display user count for channels

Fixed:

- The last word of a message sometimes dissapeared

# 2023.1-alpha1 (2023-06-30)

Added:

- First release 🎉
