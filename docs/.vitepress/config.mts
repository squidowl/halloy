import { defineConfig } from "vitepress";

export default defineConfig({
  title: "Halloy",
  description:
    "Halloy is an open-source IRC client written in Rust, with the iced GUI library. It aims to provide a simple and fast client for Mac, Windows, and Linux platforms.",
  cleanUrls: true,
  themeConfig: {
    logo: "/logo.png",
    search: {
      provider: "local",
    },
    outline: {
      level: [2, 3],
    },
    nav: [],
    editLink: {
      pattern: "https://github.com/squidowl/halloy/edit/main/docs/:path",
    },
    socialLinks: [
      { icon: "github", link: "https://github.com/squidowl/halloy" },
    ],
    sidebar: [
      {
        text: "Overview",
        collapsed: false,
        items: [
          { text: "Installation", link: "/installation" },
          { text: "Getting Started", link: "/getting-started" },
          { text: "Configuration", link: "/configuration" },
          { text: "Get in Touch", link: "/get-in-touch" },
        ],
      },
      {
        text: "Guides",
        collapsed: true,
        items: [
          { text: "Building for Flatpak", link: "/guides/flatpaks" },
          { text: "Building for macOS", link: "/guides/macos-application" },
          { text: "Connect with soju", link: "/guides/connect-with-soju" },
          { text: "Connect with ZNC", link: "/guides/connect-with-znc" },
          {
            text: "Example Server Configurations",
            link: "/guides/example-server-configurations",
          },
          {
            text: "Inclusion/Exclusion Conditions",
            link: "/configuration/conditions",
          },
          { text: "Monitor Users", link: "/guides/monitor-users" },
          { text: "Multiple Servers", link: "/guides/multiple-servers" },
          { text: "Optional Features", link: "/guides/optional-features" },
          { text: "Portable Mode", link: "/guides/portable-mode" },
          { text: "Pronunciation", link: "/guides/pronunciation" },
          { text: "Reduce Noise", link: "/guides/reduce-noise" },
          { text: "Single Pane", link: "/guides/single-pane" },
          {
            text: "Storing Passwords in a File",
            link: "/guides/password-file",
          },
          { text: "Unix Signals", link: "/guides/unix-signals" },
          { text: "Text Formatting", link: "/guides/text-formatting" },
          { text: "URL Schemes", link: "/guides/url-schemes" },
        ],
      },
      {
        text: "Configuration",
        collapsed: false,
        items: [
          {
            text: "Actions",
            link: "/configuration/actions/",
            items: [
              { text: "Buffer", link: "/configuration/actions/buffer" },
              { text: "Sidebar", link: "/configuration/actions/sidebar" },
            ],
          },
          {
            text: "Buffer",
            link: "/configuration/buffer/",
            items: [
              {
                text: "Backlog Separator",
                link: "/configuration/buffer/backlog-separator/",
              },
              {
                text: "Channel",
                link: "/configuration/buffer/channel/",
                items: [
                  {
                    text: "Message",
                    link: "/configuration/buffer/channel/message",
                  },
                  {
                    text: "Nicklist",
                    link: "/configuration/buffer/channel/nicklist",
                  },
                  {
                    text: "Typing",
                    link: "/configuration/buffer/channel/typing",
                  },
                  {
                    text: "Topic Banner",
                    link: "/configuration/buffer/channel/topic-banner",
                  },
                ],
              },
              {
                text: "Chat History",
                link: "/configuration/buffer/chat-history/",
              },
              {
                text: "Commands",
                link: "/configuration/buffer/commands/",
                items: [
                  {
                    text: "Aliases",
                    link: "/configuration/buffer/commands/aliases",
                  },
                  {
                    text: "Sysinfo",
                    link: "/configuration/buffer/commands/sysinfo",
                  },
                  { text: "Quit", link: "/configuration/buffer/commands/quit" },
                  { text: "Part", link: "/configuration/buffer/commands/part" },
                ],
              },
              {
                text: "Date Separators",
                link: "/configuration/buffer/date-separators/",
              },
              { text: "Emojis", link: "/configuration/buffer/emojis/" },
              {
                text: "Internal Messages",
                link: "/configuration/buffer/internal-messages/",
                items: [
                  {
                    text: "Error",
                    link: "/configuration/buffer/internal-messages/error",
                  },
                  {
                    text: "Success",
                    link: "/configuration/buffer/internal-messages/success",
                  },
                ],
              },
              {
                text: "Mark as Read",
                link: "/configuration/buffer/mark-as-read/",
              },
              {
                text: "Nickname",
                link: "/configuration/buffer/nickname/",
                items: [
                  {
                    text: "Hide Consecutive",
                    link: "/configuration/buffer/nickname/hide-consecutive",
                  },
                ],
              },
              {
                text: "Server Messages",
                link: "/configuration/buffer/server-messages/",
                items: [
                  {
                    text: "Condense",
                    link: "/configuration/buffer/server-messages/condense",
                  },
                ],
              },
              {
                text: "Status Message Prefix",
                link: "/configuration/buffer/status-message-prefix/",
              },
              {
                text: "Text Input",
                link: "/configuration/buffer/text-input/",
                items: [
                  {
                    text: "Autocomplete",
                    link: "/configuration/buffer/text-input/autocomplete",
                  },
                  {
                    text: "Nickname",
                    link: "/configuration/buffer/text-input/nickname",
                  },
                ],
              },
              { text: "Timestamp", link: "/configuration/buffer/timestamp/" },
              { text: "Url", link: "/configuration/buffer/url/" },
            ],
          },
          {
            text: "Check for Update on Launch",
            link: "/configuration/check-for-update-on-launch",
          },
          { text: "Commands", link: "/commands" },
          {
            text: "Context Menu",
            link: "/configuration/context-menu/",
            items: [
              { text: "Padding", link: "/configuration/context-menu/padding" },
            ],
          },
          { text: "CTCP", link: "/configuration/cctp/" },
          {
            text: "File Transfer",
            link: "/configuration/file-transfer/",
            items: [
              {
                text: "Auto Accept",
                link: "/configuration/file-transfer/auto_accept",
              },
              { text: "Server", link: "/configuration/file-transfer/server" },
            ],
          },
          { text: "Font", link: "/configuration/font/" },
          {
            text: "Highlights",
            link: "/configuration/highlights/",
            items: [
              { text: "Matches", link: "/configuration/highlights/matches" },
              { text: "Nickname", link: "/configuration/highlights/nickname" },
            ],
          },
          { text: "Keyboard", link: "/configuration/keyboard" },
          { text: "Logs", link: "/configuration/logs/" },
          { text: "Notifications", link: "/configuration/notifications/" },
          {
            text: "Pane",
            link: "/configuration/pane/",
            items: [{ text: "Gap", link: "/configuration/pane/gap" }],
          },
          {
            text: "Platform Specific",
            link: "/configuration/platform-specific/",
            items: [
              { text: "Linux", link: "/configuration/platform-specific/linux" },
              { text: "macOS", link: "/configuration/platform-specific/macos" },
              {
                text: "Windows",
                link: "/configuration/platform-specific/windows",
              },
            ],
          },
          {
            text: "Preview",
            link: "/configuration/preview/",
            items: [
              { text: "Card", link: "/configuration/preview/card" },
              { text: "Image", link: "/configuration/preview/image" },
              { text: "Request", link: "/configuration/preview/request" },
            ],
          },
          {
            text: "Proxy",
            link: "/configuration/proxy/",
            items: [
              { text: "HTTP", link: "/configuration/proxy/http" },
              { text: "SOCKS5", link: "/configuration/proxy/socks5" },
              { text: "Tor", link: "/configuration/proxy/tor" },
            ],
          },
          { text: "Scale factor", link: "/configuration/scale-factor" },
          {
            text: "Servers",
            link: "/configuration/servers/",
            items: [
              { text: "Filters", link: "/configuration/servers/filters" },
              {
                text: "SASL External",
                link: "/configuration/servers/sasl-external",
              },
              { text: "SASL Plain", link: "/configuration/servers/sasl-plain" },
              {
                text: "Confirm Message Delivery",
                link: "/configuration/servers/confirm-message-delivery",
              },
            ],
          },
          {
            text: "Sidebar",
            link: "/configuration/sidebar/",
            items: [
              { text: "Scrollbar", link: "/configuration/sidebar/scrollbar" },
              {
                text: "Unread Indicator",
                link: "/configuration/sidebar/unread-indicator",
              },
              { text: "User Menu", link: "/configuration/sidebar/user-menu" },
              { text: "Padding", link: "/configuration/sidebar/padding" },
              { text: "Spacing", link: "/configuration/sidebar/spacing" },
            ],
          },
          {
            text: "Themes",
            link: "/configuration/themes/",
            items: [
              { text: "Base16", link: "/configuration/themes/base16" },
              { text: "Community", link: "/configuration/themes/community" },
            ],
          },
          { text: "Tooltips", link: "/configuration/tooltips" },
          { text: "Window", link: "/configuration/window/" },
        ],
      },
    ],
  },
});
