// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded affix "><a href="index.html">Halloy</a></li><li class="chapter-item expanded "><a href="installation.html"><strong aria-hidden="true">1.</strong> Installation</a></li><li class="chapter-item expanded "><a href="guides/getting-started.html"><strong aria-hidden="true">2.</strong> Getting started</a></li><li class="chapter-item expanded "><a href="get-in-touch.html"><strong aria-hidden="true">3.</strong> Get in touch</a></li><li class="chapter-item expanded affix "><li class="part-title">Guides</li><li class="chapter-item expanded "><a href="guides/connect-with-soju.html"><strong aria-hidden="true">4.</strong> Connect with soju</a></li><li class="chapter-item expanded "><a href="guides/connect-with-znc.html"><strong aria-hidden="true">5.</strong> Connect with ZNC</a></li><li class="chapter-item expanded "><a href="guides/portable-mode.html"><strong aria-hidden="true">6.</strong> Portable mode</a></li><li class="chapter-item expanded "><a href="guides/multiple-servers.html"><strong aria-hidden="true">7.</strong> Multiple servers</a></li><li class="chapter-item expanded "><a href="guides/password-file.html"><strong aria-hidden="true">8.</strong> Storing passwords in a File</a></li><li class="chapter-item expanded "><a href="guides/text-formatting.html"><strong aria-hidden="true">9.</strong> Text Formatting</a></li><li class="chapter-item expanded "><a href="guides/monitor-users.html"><strong aria-hidden="true">10.</strong> Monitor users</a></li><li class="chapter-item expanded affix "><li class="part-title">Configuration</li><li class="chapter-item expanded "><a href="configuration/index.html"><strong aria-hidden="true">11.</strong> Configuration</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="configuration/buffer/index.html"><strong aria-hidden="true">11.1.</strong> Buffer</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="configuration/buffer/away.html"><strong aria-hidden="true">11.1.1.</strong> Away</a></li><li class="chapter-item expanded "><a href="configuration/buffer/channel/index.html"><strong aria-hidden="true">11.1.2.</strong> Channel</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="configuration/buffer/channel/nicklist.html"><strong aria-hidden="true">11.1.2.1.</strong> Nicklist</a></li><li class="chapter-item expanded "><a href="configuration/buffer/channel/message.html"><strong aria-hidden="true">11.1.2.2.</strong> Message</a></li><li class="chapter-item expanded "><a href="configuration/buffer/channel/topic.html"><strong aria-hidden="true">11.1.2.3.</strong> Topic</a></li></ol></li><li class="chapter-item expanded "><a href="configuration/buffer/commands.html"><strong aria-hidden="true">11.1.3.</strong> Commands</a></li><li class="chapter-item expanded "><a href="configuration/buffer/internal_messages/index.html"><strong aria-hidden="true">11.1.4.</strong> Internal Messages</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="configuration/buffer/internal_messages/success.html"><strong aria-hidden="true">11.1.4.1.</strong> Success</a></li><li class="chapter-item expanded "><a href="configuration/buffer/internal_messages/error.html"><strong aria-hidden="true">11.1.4.2.</strong> Error</a></li></ol></li><li class="chapter-item expanded "><a href="configuration/buffer/nickname.html"><strong aria-hidden="true">11.1.5.</strong> Nickname</a></li><li class="chapter-item expanded "><a href="configuration/buffer/server_messages/index.html"><strong aria-hidden="true">11.1.6.</strong> Server Messages</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="configuration/buffer/server_messages/change_host.html"><strong aria-hidden="true">11.1.6.1.</strong> Change Host</a></li><li class="chapter-item expanded "><a href="configuration/buffer/server_messages/join.html"><strong aria-hidden="true">11.1.6.2.</strong> Join</a></li><li class="chapter-item expanded "><a href="configuration/buffer/server_messages/monitored_offline.html"><strong aria-hidden="true">11.1.6.3.</strong> Monitored Offline</a></li><li class="chapter-item expanded "><a href="configuration/buffer/server_messages/monitored_online.html"><strong aria-hidden="true">11.1.6.4.</strong> Monitored Online</a></li><li class="chapter-item expanded "><a href="configuration/buffer/server_messages/part.html"><strong aria-hidden="true">11.1.6.5.</strong> Part</a></li><li class="chapter-item expanded "><a href="configuration/buffer/server_messages/quit.html"><strong aria-hidden="true">11.1.6.6.</strong> Quit</a></li><li class="chapter-item expanded "><a href="configuration/buffer/server_messages/standard_reply_fail.html"><strong aria-hidden="true">11.1.6.7.</strong> Standard Reply: Fail</a></li><li class="chapter-item expanded "><a href="configuration/buffer/server_messages/standard_reply_note.html"><strong aria-hidden="true">11.1.6.8.</strong> Standard Reply: Note</a></li><li class="chapter-item expanded "><a href="configuration/buffer/server_messages/standard_reply_warn.html"><strong aria-hidden="true">11.1.6.9.</strong> Standard Reply: Warn</a></li><li class="chapter-item expanded "><a href="configuration/buffer/server_messages/topic.html"><strong aria-hidden="true">11.1.6.10.</strong> Topic</a></li></ol></li><li class="chapter-item expanded "><a href="configuration/buffer/text_input/index.html"><strong aria-hidden="true">11.1.7.</strong> Text Input</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="configuration/buffer/text_input/autocomplete.html"><strong aria-hidden="true">11.1.7.1.</strong> Autocomplete</a></li></ol></li><li class="chapter-item expanded "><a href="configuration/buffer/timestamp.html"><strong aria-hidden="true">11.1.8.</strong> Timestamp</a></li><li class="chapter-item expanded "><a href="configuration/buffer/date_separators.html"><strong aria-hidden="true">11.1.9.</strong> Date Separators</a></li><li class="chapter-item expanded "><a href="configuration/buffer/chat_history.html"><strong aria-hidden="true">11.1.10.</strong> Chat History</a></li></ol></li><li class="chapter-item expanded "><a href="configuration/file_transfer/index.html"><strong aria-hidden="true">11.2.</strong> File Transfer</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="configuration/file_transfer/server.html"><strong aria-hidden="true">11.2.1.</strong> Server</a></li></ol></li><li class="chapter-item expanded "><a href="configuration/font.html"><strong aria-hidden="true">11.3.</strong> Font</a></li><li class="chapter-item expanded "><a href="configuration/keyboard.html"><strong aria-hidden="true">11.4.</strong> Keyboard</a></li><li class="chapter-item expanded "><a href="configuration/notifications.html"><strong aria-hidden="true">11.5.</strong> Notifications</a></li><li class="chapter-item expanded "><a href="configuration/pane/index.html"><strong aria-hidden="true">11.6.</strong> Pane</a></li><li class="chapter-item expanded "><a href="configuration/proxy.html"><strong aria-hidden="true">11.7.</strong> Proxy</a></li><li class="chapter-item expanded "><a href="configuration/preview.html"><strong aria-hidden="true">11.8.</strong> Preview</a></li><li class="chapter-item expanded "><a href="configuration/scale-factor.html"><strong aria-hidden="true">11.9.</strong> Scale factor</a></li><li class="chapter-item expanded "><a href="configuration/servers/index.html"><strong aria-hidden="true">11.10.</strong> Servers</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="configuration/servers/sasl/index.html"><strong aria-hidden="true">11.10.1.</strong> SASL</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="configuration/servers/sasl/plain.html"><strong aria-hidden="true">11.10.1.1.</strong> Plain</a></li><li class="chapter-item expanded "><a href="configuration/servers/sasl/external.html"><strong aria-hidden="true">11.10.1.2.</strong> External</a></li></ol></li></ol></li><li class="chapter-item expanded "><a href="configuration/sidebar/index.html"><strong aria-hidden="true">11.11.</strong> Sidebar</a></li><li class="chapter-item expanded "><a href="configuration/themes/index.html"><strong aria-hidden="true">11.12.</strong> Themes</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="configuration/themes/community.html"><strong aria-hidden="true">11.12.1.</strong> Community</a></li><li class="chapter-item expanded "><a href="configuration/themes/base16.html"><strong aria-hidden="true">11.12.2.</strong> Base16</a></li></ol></li><li class="chapter-item expanded "><a href="configuration/tooltips.html"><strong aria-hidden="true">11.13.</strong> Tooltips</a></li></ol></li><li class="chapter-item expanded "><a href="url-schemes.html"><strong aria-hidden="true">12.</strong> URL Schemes</a></li><li class="chapter-item expanded "><a href="commands.html"><strong aria-hidden="true">13.</strong> Commands</a></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split("#")[0];
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
