import footnote from "markdown-it-footnote";
import { defineConfig } from "vitepress";

const docsBase = process.env.DOCS_BASE ?? "/";
const docsOutDir = process.env.DOCS_OUT_DIR ?? ".vitepress/dist";
const docsChannel = process.env.DOCS_CHANNEL ?? "stable";
const stableLabel = process.env.DOCS_STABLE_LABEL ?? "Latest";
const nightlyLabel = process.env.DOCS_NIGHTLY_LABEL ?? "Nightly";
const stableUrl = process.env.DOCS_STABLE_URL ?? "/";
const nightlyUrl = process.env.DOCS_NIGHTLY_URL ?? "/nightly/";
const channelLabel = docsChannel === "nightly" ? nightlyLabel : stableLabel;

export default defineConfig({
  title: "Halloy",
  description:
    "Halloy is an open-source IRC client written in Rust, with the iced GUI library. It aims to provide a simple and fast client for Mac, Windows, and Linux platforms.",
  base: docsBase,
  outDir: docsOutDir,
  appearance: "force-dark",
  cleanUrls: true,
  head: [["link", { rel: "icon", type: "image/png", href: "/favicon.png" }]],
  markdown: {
    config: (md) => {
      md.use(footnote);
    },
    container: {
      tipLabel: '💡 Tip',
      warningLabel: '⚠️ Warning',
      dangerLabel: '⚠️ Danger',
      infoLabel: '💡 Info',
      detailsLabel: 'Details'
    },
  },
  themeConfig: {
    logo: "/logo.png",
    search: {
      provider: "local",
    },
    outline: {
      level: [2, 4],
    },
    nav: [
      {
        text: channelLabel,
        items: [
          { text: stableLabel, link: stableUrl },
          { text: nightlyLabel, link: nightlyUrl },
        ],
      },
      {
        text: "Themes",
        link: "https://themes.halloy.chat/",
      },
    ],
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
          { text: "Custom Themes", link: "/guides/custom-themes" },
          { text: "Exec Command", link: "/guides/exec-command" },
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
            link: "/configuration/actions",
          },
          {
            text: "Buffer",
            link: "/configuration/buffer",
          },
          {
            text: "Check for Update on Launch",
            link: "/configuration/check-for-update-on-launch",
          },
          { text: "Commands", link: "/commands" },
          {
            text: "Context Menu",
            link: "/configuration/context-menu",
          },
          { text: "CTCP", link: "/configuration/cctp" },
          {
            text: "File Transfer",
            link: "/configuration/file-transfer",
          },
          { text: "Font", link: "/configuration/font" },
          {
            text: "Highlights",
            link: "/configuration/highlights",
          },
          { text: "Keyboard", link: "/configuration/keyboard" },
          { text: "Logs", link: "/configuration/logs" },
          { text: "Notifications", link: "/configuration/notifications" },
          {
            text: "Pane",
            link: "/configuration/pane",
          },
          {
            text: "Platform Specific",
            link: "/configuration/platform-specific",
          },
          {
            text: "Preview",
            link: "/configuration/preview",
          },
          {
            text: "Proxy",
            link: "/configuration/proxy",
          },
          { text: "Scale factor", link: "/configuration/scale-factor" },
          {
            text: "Servers",
            link: "/configuration/servers",
          },
          {
            text: "Sidebar",
            link: "/configuration/sidebar",
          },
          {
            text: "Themes",
            link: "/configuration/themes",
          },
          { text: "Tooltips", link: "/configuration/tooltips" },
          { text: "Window", link: "/configuration/window" },
        ],
      },
    ],
  },
});
