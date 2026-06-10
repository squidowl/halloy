import footnote from "markdown-it-footnote";
import { defineConfig } from "vitepress";

const releaseLabel = process.env.DOCS_VERSION ?? "Latest Release";
const releaseLink =
  process.env.DOCS_CHANNEL === "nightly" ? "https://halloy.chat" : "/";
const nightlyLabel = process.env.DOCS_SHA
  ? `Nightly (${process.env.DOCS_SHA.slice(0, 7)})`
  : "Nightly";
const nightlyLink =
  process.env.DOCS_CHANNEL === "nightly" ? "/" : "https://nightly.halloy.chat";
const docsChannel =
  process.env.DOCS_CHANNEL === "nightly" ? nightlyLabel : releaseLabel;

const guidesItems = [
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
  { text: "File Uploads", link: "/guides/filehost" },
  { text: "Unix Signals", link: "/guides/unix-signals" },
  { text: "Text Formatting", link: "/guides/text-formatting" },
  { text: "URL Schemes", link: "/guides/url-schemes" },
];

const configurationItems = [
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
  { text: "Display", link: "/configuration/display" },
  {
    text: "File Transfer",
    link: "/configuration/file-transfer",
  },
  {
    text: "File Upload",
    link: "/configuration/file-upload",
  },
  { text: "Font", link: "/configuration/font" },
  {
    text: "Highlights",
    link: "/configuration/highlights",
  },
  { text: "Keyboard", link: "/configuration/keyboard" },
  { text: "Logs", link: "/configuration/logs" },
  { text: "Metadata", link: "/configuration/metadata" },
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
  { text: "Runtime", link: "/configuration/runtime" },
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
];

export default defineConfig({
  title: "Halloy",
  description:
    "Halloy is an open-source IRC client written in Rust, with the iced GUI library. It aims to provide a simple and fast client for Mac, Windows, and Linux platforms.",
  base: process.env.DOCS_BASE ?? "/",
  appearance: "force-dark",
  cleanUrls: true,
  head: [["link", { rel: "icon", type: "image/png", href: "/favicon.png" }]],
  markdown: {
    config: (md) => {
      md.use(footnote);
    },
    container: {
      tipLabel: "💡 Tip",
      warningLabel: "⚠️ Warning",
      dangerLabel: "⚠️ Danger",
      infoLabel: "💡 Info",
      detailsLabel: "Details",
    },
  },
  themeConfig: {
    logo: "/logo.png",
    siteTitle: `Halloy <span class="VPBadge info mobile-only">${releaseLabel}</span>`,
    search: {
      provider: "local",
    },
    outline: {
      level: [2, 4],
    },
    nav: [
      {
        text: docsChannel,
        items: [
          { text: releaseLabel, link: releaseLink, target: "_self" },
          { text: nightlyLabel, link: nightlyLink, target: "_self" },
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
    sidebar: {
      "/configuration": [
        {
          items: [
            { text: "Installation", link: "/installation" },
            { text: "Getting Started", link: "/getting-started" },
            {
              text: "Configuration",
              link: "/configuration",
              collapsed: false,
              items: configurationItems,
            },
            {
              text: "Guides",
              collapsed: false,
              items: guidesItems,
            },
            { text: "Contributing", link: "/contributing" },
            { text: "Get in Touch", link: "/get-in-touch" },
          ],
        },
      ],
      "/": [
        {
          items: [
            { text: "Installation", link: "/installation" },
            { text: "Getting Started", link: "/getting-started" },
            {
              text: "Configuration",
              link: "/configuration",
              collapsed: true,
              items: configurationItems,
            },
            {
              text: "Guides",
              collapsed: false,
              items: guidesItems,
            },
            { text: "Contributing", link: "/contributing" },
            { text: "Get in Touch", link: "/get-in-touch" },
          ],
        },
      ],
    },
  },
});
