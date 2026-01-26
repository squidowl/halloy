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
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="index.html">Halloy</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="installation.html"><strong aria-hidden="true">1.</strong> Installation</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="getting-started.html"><strong aria-hidden="true">2.</strong> Getting Started</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration.html"><strong aria-hidden="true">3.</strong> Configuration</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="get-in-touch.html"><strong aria-hidden="true">4.</strong> Get in Touch</a></span></li><li class="chapter-item expanded "><li class="part-title">Guides</li></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="guides/flatpaks.html"><strong aria-hidden="true">5.</strong> Building for Flatpak</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="guides/macos-application.html"><strong aria-hidden="true">6.</strong> Building for macOS</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="guides/connect-with-soju.html"><strong aria-hidden="true">7.</strong> Connect with soju</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="guides/connect-with-znc.html"><strong aria-hidden="true">8.</strong> Connect with ZNC</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="guides/example-server-configurations.html"><strong aria-hidden="true">9.</strong> Example Server Configurations</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/conditions.html"><strong aria-hidden="true">10.</strong> Inclusion/Exclusion Conditions</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="guides/monitor-users.html"><strong aria-hidden="true">11.</strong> Monitor Users</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="guides/multiple-servers.html"><strong aria-hidden="true">12.</strong> Multiple Servers</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="guides/optional-features.html"><strong aria-hidden="true">13.</strong> Optional Features</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="guides/portable-mode.html"><strong aria-hidden="true">14.</strong> Portable Mode</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="guides/pronunciation.html"><strong aria-hidden="true">15.</strong> Pronunciation</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="guides/reduce-noise.html"><strong aria-hidden="true">16.</strong> Reduce Noise</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="guides/single-pane.html"><strong aria-hidden="true">17.</strong> Single Pane</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="guides/password-file.html"><strong aria-hidden="true">18.</strong> Storing Passwords in a File</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="guides/unix-signals.html"><strong aria-hidden="true">19.</strong> Unix Signals</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="guides/text-formatting.html"><strong aria-hidden="true">20.</strong> Text Formatting</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="guides/url-schemes.html"><strong aria-hidden="true">21.</strong> URL Schemes</a></span></li><li class="chapter-item expanded "><li class="part-title">Configuration</li></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/actions/index.html"><strong aria-hidden="true">22.</strong> Actions</a></span><ol class="section"><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/actions/buffer.html"><strong aria-hidden="true">22.1.</strong> Buffer</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/actions/sidebar.html"><strong aria-hidden="true">22.2.</strong> Sidebar</a></span></li></ol><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/index.html"><strong aria-hidden="true">23.</strong> Buffer</a></span><ol class="section"><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/backlog-separator/index.html"><strong aria-hidden="true">23.1.</strong> Backlog Separator</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/channel/index.html"><strong aria-hidden="true">23.2.</strong> Channel</a></span><ol class="section"><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/channel/message.html"><strong aria-hidden="true">23.2.1.</strong> Message</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/channel/nicklist.html"><strong aria-hidden="true">23.2.2.</strong> Nicklist</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/channel/topic-banner.html"><strong aria-hidden="true">23.2.3.</strong> Topic Banner</a></span></li></ol><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/chat-history/index.html"><strong aria-hidden="true">23.3.</strong> Chat History</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/commands/index.html"><strong aria-hidden="true">23.4.</strong> Commands</a></span><ol class="section"><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/commands/sysinfo.html"><strong aria-hidden="true">23.4.1.</strong> Sysinfo</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/commands/quit.html"><strong aria-hidden="true">23.4.2.</strong> Quit</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/commands/part.html"><strong aria-hidden="true">23.4.3.</strong> Part</a></span></li></ol><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/date-separators/index.html"><strong aria-hidden="true">23.5.</strong> Date Separators</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/emojis/index.html"><strong aria-hidden="true">23.6.</strong> Emojis</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/internal-messages/index.html"><strong aria-hidden="true">23.7.</strong> Internal Messages</a></span><ol class="section"><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/internal-messages/error.html"><strong aria-hidden="true">23.7.1.</strong> Error</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/internal-messages/success.html"><strong aria-hidden="true">23.7.2.</strong> Success</a></span></li></ol><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/mark-as-read/index.html"><strong aria-hidden="true">23.8.</strong> Mark as Read</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/nickname/index.html"><strong aria-hidden="true">23.9.</strong> Nickname</a></span><ol class="section"><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/nickname/hide-consecutive.html"><strong aria-hidden="true">23.9.1.</strong> Hide Consecutive</a></span></li></ol><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/server-messages/index.html"><strong aria-hidden="true">23.10.</strong> Server Messages</a></span><ol class="section"><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/server-messages/condense.html"><strong aria-hidden="true">23.10.1.</strong> Condense</a></span></li></ol><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/status-message-prefix/index.html"><strong aria-hidden="true">23.11.</strong> Status Message Prefix</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/text-input/index.html"><strong aria-hidden="true">23.12.</strong> Text Input</a></span><ol class="section"><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/text-input/autocomplete.html"><strong aria-hidden="true">23.12.1.</strong> Autocomplete</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/text-input/nickname.html"><strong aria-hidden="true">23.12.2.</strong> Nickname</a></span></li></ol><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/timestamp/index.html"><strong aria-hidden="true">23.13.</strong> Timestamp</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/buffer/url/index.html"><strong aria-hidden="true">23.14.</strong> Url</a></span></li></ol><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="commands.html"><strong aria-hidden="true">24.</strong> Commands</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/context-menu/index.html"><strong aria-hidden="true">25.</strong> Context Menu</a></span><ol class="section"><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/context-menu/padding.html"><strong aria-hidden="true">25.1.</strong> Padding</a></span></li></ol><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/cctp/index.html"><strong aria-hidden="true">26.</strong> CTCP</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/file-transfer/index.html"><strong aria-hidden="true">27.</strong> File Transfer</a></span><ol class="section"><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/file-transfer/auto_accept.html"><strong aria-hidden="true">27.1.</strong> Auto Accept</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/file-transfer/server.html"><strong aria-hidden="true">27.2.</strong> Server</a></span></li></ol><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/font/index.html"><strong aria-hidden="true">28.</strong> Font</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/highlights/index.html"><strong aria-hidden="true">29.</strong> Highlights</a></span><ol class="section"><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/highlights/matches.html"><strong aria-hidden="true">29.1.</strong> Matches</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/highlights/nickname.html"><strong aria-hidden="true">29.2.</strong> Nickname</a></span></li></ol><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/keyboard.html"><strong aria-hidden="true">30.</strong> Keyboard</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/logs/index.html"><strong aria-hidden="true">31.</strong> Logs</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/notifications/index.html"><strong aria-hidden="true">32.</strong> Notifications</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/pane/index.html"><strong aria-hidden="true">33.</strong> Pane</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/platform-specific/index.html"><strong aria-hidden="true">34.</strong> Platform Specific</a></span><ol class="section"><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/platform-specific/linux.html"><strong aria-hidden="true">34.1.</strong> Linux</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/platform-specific/macos.html"><strong aria-hidden="true">34.2.</strong> macOS</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/platform-specific/windows.html"><strong aria-hidden="true">34.3.</strong> Windows</a></span></li></ol><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/preview/index.html"><strong aria-hidden="true">35.</strong> Preview</a></span><ol class="section"><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/preview/card.html"><strong aria-hidden="true">35.1.</strong> Card</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/preview/image.html"><strong aria-hidden="true">35.2.</strong> Image</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/preview/request.html"><strong aria-hidden="true">35.3.</strong> Request</a></span></li></ol><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/proxy/index.html"><strong aria-hidden="true">36.</strong> Proxy</a></span><ol class="section"><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/proxy/http.html"><strong aria-hidden="true">36.1.</strong> HTTP</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/proxy/socks5.html"><strong aria-hidden="true">36.2.</strong> SOCKS5</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/proxy/tor.html"><strong aria-hidden="true">36.3.</strong> Tor</a></span></li></ol><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/scale-factor.html"><strong aria-hidden="true">37.</strong> Scale factor</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/servers/index.html"><strong aria-hidden="true">38.</strong> Servers</a></span><ol class="section"><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/servers/filters.html"><strong aria-hidden="true">38.1.</strong> Filters</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/servers/sasl-external.html"><strong aria-hidden="true">38.2.</strong> SASL External</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/servers/sasl-plain.html"><strong aria-hidden="true">38.3.</strong> SASL Plain</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/servers/confirm-message-delivery.html"><strong aria-hidden="true">38.4.</strong> Confirm Message Delivery</a></span></li></ol><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/sidebar/index.html"><strong aria-hidden="true">39.</strong> Sidebar</a></span><ol class="section"><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/sidebar/scrollbar.html"><strong aria-hidden="true">39.1.</strong> Scrollbar</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/sidebar/unread-indicator.html"><strong aria-hidden="true">39.2.</strong> Unread Indicator</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/sidebar/user-menu.html"><strong aria-hidden="true">39.3.</strong> User Menu</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/sidebar/padding.html"><strong aria-hidden="true">39.4.</strong> Padding</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/sidebar/spacing.html"><strong aria-hidden="true">39.5.</strong> Spacing</a></span></li></ol><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/themes/index.html"><strong aria-hidden="true">40.</strong> Themes</a></span><ol class="section"><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/themes/base16.html"><strong aria-hidden="true">40.1.</strong> Base16</a></span></li><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/themes/community.html"><strong aria-hidden="true">40.2.</strong> Community</a></span></li></ol><li class="chapter-item expanded "><span class="chapter-link-wrapper"><a href="configuration/tooltips.html"><strong aria-hidden="true">41.</strong> Tooltips</a></span></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split('#')[0].split('?')[0];
        if (current_page.endsWith('/')) {
            current_page += 'index.html';
        }
        const links = Array.prototype.slice.call(this.querySelectorAll('a'));
        const l = links.length;
        for (let i = 0; i < l; ++i) {
            const link = links[i];
            const href = link.getAttribute('href');
            if (href && !href.startsWith('#') && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The 'index' page is supposed to alias the first chapter in the book.
            if (link.href === current_page
                || i === 0
                && path_to_root === ''
                && current_page.endsWith('/index.html')) {
                link.classList.add('active');
                let parent = link.parentElement;
                while (parent) {
                    if (parent.tagName === 'LI' && parent.classList.contains('chapter-item')) {
                        parent.classList.add('expanded');
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', e => {
            if (e.target.tagName === 'A') {
                const clientRect = e.target.getBoundingClientRect();
                const sidebarRect = this.getBoundingClientRect();
                sessionStorage.setItem('sidebar-scroll-offset', clientRect.top - sidebarRect.top);
            }
        }, { passive: true });
        const sidebarScrollOffset = sessionStorage.getItem('sidebar-scroll-offset');
        sessionStorage.removeItem('sidebar-scroll-offset');
        if (sidebarScrollOffset !== null) {
            // preserve sidebar scroll position when navigating via links within sidebar
            const activeSection = this.querySelector('.active');
            if (activeSection) {
                const clientRect = activeSection.getBoundingClientRect();
                const sidebarRect = this.getBoundingClientRect();
                const currentOffset = clientRect.top - sidebarRect.top;
                this.scrollTop += currentOffset - parseFloat(sidebarScrollOffset);
            }
        } else {
            // scroll sidebar to current active section when navigating via
            // 'next/previous chapter' buttons
            const activeSection = document.querySelector('#mdbook-sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        const sidebarAnchorToggles = document.querySelectorAll('.chapter-fold-toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(el => {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define('mdbook-sidebar-scrollbox', MDBookSidebarScrollbox);


// ---------------------------------------------------------------------------
// Support for dynamically adding headers to the sidebar.

(function() {
    // This is used to detect which direction the page has scrolled since the
    // last scroll event.
    let lastKnownScrollPosition = 0;
    // This is the threshold in px from the top of the screen where it will
    // consider a header the "current" header when scrolling down.
    const defaultDownThreshold = 150;
    // Same as defaultDownThreshold, except when scrolling up.
    const defaultUpThreshold = 300;
    // The threshold is a virtual horizontal line on the screen where it
    // considers the "current" header to be above the line. The threshold is
    // modified dynamically to handle headers that are near the bottom of the
    // screen, and to slightly offset the behavior when scrolling up vs down.
    let threshold = defaultDownThreshold;
    // This is used to disable updates while scrolling. This is needed when
    // clicking the header in the sidebar, which triggers a scroll event. It
    // is somewhat finicky to detect when the scroll has finished, so this
    // uses a relatively dumb system of disabling scroll updates for a short
    // time after the click.
    let disableScroll = false;
    // Array of header elements on the page.
    let headers;
    // Array of li elements that are initially collapsed headers in the sidebar.
    // I'm not sure why eslint seems to have a false positive here.
    // eslint-disable-next-line prefer-const
    let headerToggles = [];
    // This is a debugging tool for the threshold which you can enable in the console.
    let thresholdDebug = false;

    // Updates the threshold based on the scroll position.
    function updateThreshold() {
        const scrollTop = window.pageYOffset || document.documentElement.scrollTop;
        const windowHeight = window.innerHeight;
        const documentHeight = document.documentElement.scrollHeight;

        // The number of pixels below the viewport, at most documentHeight.
        // This is used to push the threshold down to the bottom of the page
        // as the user scrolls towards the bottom.
        const pixelsBelow = Math.max(0, documentHeight - (scrollTop + windowHeight));
        // The number of pixels above the viewport, at least defaultDownThreshold.
        // Similar to pixelsBelow, this is used to push the threshold back towards
        // the top when reaching the top of the page.
        const pixelsAbove = Math.max(0, defaultDownThreshold - scrollTop);
        // How much the threshold should be offset once it gets close to the
        // bottom of the page.
        const bottomAdd = Math.max(0, windowHeight - pixelsBelow - defaultDownThreshold);
        let adjustedBottomAdd = bottomAdd;

        // Adjusts bottomAdd for a small document. The calculation above
        // assumes the document is at least twice the windowheight in size. If
        // it is less than that, then bottomAdd needs to be shrunk
        // proportional to the difference in size.
        if (documentHeight < windowHeight * 2) {
            const maxPixelsBelow = documentHeight - windowHeight;
            const t = 1 - pixelsBelow / Math.max(1, maxPixelsBelow);
            const clamp = Math.max(0, Math.min(1, t));
            adjustedBottomAdd *= clamp;
        }

        let scrollingDown = true;
        if (scrollTop < lastKnownScrollPosition) {
            scrollingDown = false;
        }

        if (scrollingDown) {
            // When scrolling down, move the threshold up towards the default
            // downwards threshold position. If near the bottom of the page,
            // adjustedBottomAdd will offset the threshold towards the bottom
            // of the page.
            const amountScrolledDown = scrollTop - lastKnownScrollPosition;
            const adjustedDefault = defaultDownThreshold + adjustedBottomAdd;
            threshold = Math.max(adjustedDefault, threshold - amountScrolledDown);
        } else {
            // When scrolling up, move the threshold down towards the default
            // upwards threshold position. If near the bottom of the page,
            // quickly transition the threshold back up where it normally
            // belongs.
            const amountScrolledUp = lastKnownScrollPosition - scrollTop;
            const adjustedDefault = defaultUpThreshold - pixelsAbove
                + Math.max(0, adjustedBottomAdd - defaultDownThreshold);
            threshold = Math.min(adjustedDefault, threshold + amountScrolledUp);
        }

        if (documentHeight <= windowHeight) {
            threshold = 0;
        }

        if (thresholdDebug) {
            const id = 'mdbook-threshold-debug-data';
            let data = document.getElementById(id);
            if (data === null) {
                data = document.createElement('div');
                data.id = id;
                data.style.cssText = `
                    position: fixed;
                    top: 50px;
                    right: 10px;
                    background-color: 0xeeeeee;
                    z-index: 9999;
                    pointer-events: none;
                `;
                document.body.appendChild(data);
            }
            data.innerHTML = `
                <table>
                  <tr><td>documentHeight</td><td>${documentHeight.toFixed(1)}</td></tr>
                  <tr><td>windowHeight</td><td>${windowHeight.toFixed(1)}</td></tr>
                  <tr><td>scrollTop</td><td>${scrollTop.toFixed(1)}</td></tr>
                  <tr><td>pixelsAbove</td><td>${pixelsAbove.toFixed(1)}</td></tr>
                  <tr><td>pixelsBelow</td><td>${pixelsBelow.toFixed(1)}</td></tr>
                  <tr><td>bottomAdd</td><td>${bottomAdd.toFixed(1)}</td></tr>
                  <tr><td>adjustedBottomAdd</td><td>${adjustedBottomAdd.toFixed(1)}</td></tr>
                  <tr><td>scrollingDown</td><td>${scrollingDown}</td></tr>
                  <tr><td>threshold</td><td>${threshold.toFixed(1)}</td></tr>
                </table>
            `;
            drawDebugLine();
        }

        lastKnownScrollPosition = scrollTop;
    }

    function drawDebugLine() {
        if (!document.body) {
            return;
        }
        const id = 'mdbook-threshold-debug-line';
        const existingLine = document.getElementById(id);
        if (existingLine) {
            existingLine.remove();
        }
        const line = document.createElement('div');
        line.id = id;
        line.style.cssText = `
            position: fixed;
            top: ${threshold}px;
            left: 0;
            width: 100vw;
            height: 2px;
            background-color: red;
            z-index: 9999;
            pointer-events: none;
        `;
        document.body.appendChild(line);
    }

    function mdbookEnableThresholdDebug() {
        thresholdDebug = true;
        updateThreshold();
        drawDebugLine();
    }

    window.mdbookEnableThresholdDebug = mdbookEnableThresholdDebug;

    // Updates which headers in the sidebar should be expanded. If the current
    // header is inside a collapsed group, then it, and all its parents should
    // be expanded.
    function updateHeaderExpanded(currentA) {
        // Add expanded to all header-item li ancestors.
        let current = currentA.parentElement;
        while (current) {
            if (current.tagName === 'LI' && current.classList.contains('header-item')) {
                current.classList.add('expanded');
            }
            current = current.parentElement;
        }
    }

    // Updates which header is marked as the "current" header in the sidebar.
    // This is done with a virtual Y threshold, where headers at or below
    // that line will be considered the current one.
    function updateCurrentHeader() {
        if (!headers || !headers.length) {
            return;
        }

        // Reset the classes, which will be rebuilt below.
        const els = document.getElementsByClassName('current-header');
        for (const el of els) {
            el.classList.remove('current-header');
        }
        for (const toggle of headerToggles) {
            toggle.classList.remove('expanded');
        }

        // Find the last header that is above the threshold.
        let lastHeader = null;
        for (const header of headers) {
            const rect = header.getBoundingClientRect();
            if (rect.top <= threshold) {
                lastHeader = header;
            } else {
                break;
            }
        }
        if (lastHeader === null) {
            lastHeader = headers[0];
            const rect = lastHeader.getBoundingClientRect();
            const windowHeight = window.innerHeight;
            if (rect.top >= windowHeight) {
                return;
            }
        }

        // Get the anchor in the summary.
        const href = '#' + lastHeader.id;
        const a = [...document.querySelectorAll('.header-in-summary')]
            .find(element => element.getAttribute('href') === href);
        if (!a) {
            return;
        }

        a.classList.add('current-header');

        updateHeaderExpanded(a);
    }

    // Updates which header is "current" based on the threshold line.
    function reloadCurrentHeader() {
        if (disableScroll) {
            return;
        }
        updateThreshold();
        updateCurrentHeader();
    }


    // When clicking on a header in the sidebar, this adjusts the threshold so
    // that it is located next to the header. This is so that header becomes
    // "current".
    function headerThresholdClick(event) {
        // See disableScroll description why this is done.
        disableScroll = true;
        setTimeout(() => {
            disableScroll = false;
        }, 100);
        // requestAnimationFrame is used to delay the update of the "current"
        // header until after the scroll is done, and the header is in the new
        // position.
        requestAnimationFrame(() => {
            requestAnimationFrame(() => {
                // Closest is needed because if it has child elements like <code>.
                const a = event.target.closest('a');
                const href = a.getAttribute('href');
                const targetId = href.substring(1);
                const targetElement = document.getElementById(targetId);
                if (targetElement) {
                    threshold = targetElement.getBoundingClientRect().bottom;
                    updateCurrentHeader();
                }
            });
        });
    }

    // Takes the nodes from the given head and copies them over to the
    // destination, along with some filtering.
    function filterHeader(source, dest) {
        const clone = source.cloneNode(true);
        clone.querySelectorAll('mark').forEach(mark => {
            mark.replaceWith(...mark.childNodes);
        });
        dest.append(...clone.childNodes);
    }

    // Scans page for headers and adds them to the sidebar.
    document.addEventListener('DOMContentLoaded', function() {
        const activeSection = document.querySelector('#mdbook-sidebar .active');
        if (activeSection === null) {
            return;
        }

        const main = document.getElementsByTagName('main')[0];
        headers = Array.from(main.querySelectorAll('h2, h3, h4, h5, h6'))
            .filter(h => h.id !== '' && h.children.length && h.children[0].tagName === 'A');

        if (headers.length === 0) {
            return;
        }

        // Build a tree of headers in the sidebar.

        const stack = [];

        const firstLevel = parseInt(headers[0].tagName.charAt(1));
        for (let i = 1; i < firstLevel; i++) {
            const ol = document.createElement('ol');
            ol.classList.add('section');
            if (stack.length > 0) {
                stack[stack.length - 1].ol.appendChild(ol);
            }
            stack.push({level: i + 1, ol: ol});
        }

        // The level where it will start folding deeply nested headers.
        const foldLevel = 3;

        for (let i = 0; i < headers.length; i++) {
            const header = headers[i];
            const level = parseInt(header.tagName.charAt(1));

            const currentLevel = stack[stack.length - 1].level;
            if (level > currentLevel) {
                // Begin nesting to this level.
                for (let nextLevel = currentLevel + 1; nextLevel <= level; nextLevel++) {
                    const ol = document.createElement('ol');
                    ol.classList.add('section');
                    const last = stack[stack.length - 1];
                    const lastChild = last.ol.lastChild;
                    // Handle the case where jumping more than one nesting
                    // level, which doesn't have a list item to place this new
                    // list inside of.
                    if (lastChild) {
                        lastChild.appendChild(ol);
                    } else {
                        last.ol.appendChild(ol);
                    }
                    stack.push({level: nextLevel, ol: ol});
                }
            } else if (level < currentLevel) {
                while (stack.length > 1 && stack[stack.length - 1].level > level) {
                    stack.pop();
                }
            }

            const li = document.createElement('li');
            li.classList.add('header-item');
            li.classList.add('expanded');
            if (level < foldLevel) {
                li.classList.add('expanded');
            }
            const span = document.createElement('span');
            span.classList.add('chapter-link-wrapper');
            const a = document.createElement('a');
            span.appendChild(a);
            a.href = '#' + header.id;
            a.classList.add('header-in-summary');
            filterHeader(header.children[0], a);
            a.addEventListener('click', headerThresholdClick);
            const nextHeader = headers[i + 1];
            if (nextHeader !== undefined) {
                const nextLevel = parseInt(nextHeader.tagName.charAt(1));
                if (nextLevel > level && level >= foldLevel) {
                    const toggle = document.createElement('a');
                    toggle.classList.add('chapter-fold-toggle');
                    toggle.classList.add('header-toggle');
                    toggle.addEventListener('click', () => {
                        li.classList.toggle('expanded');
                    });
                    const toggleDiv = document.createElement('div');
                    toggleDiv.textContent = '❱';
                    toggle.appendChild(toggleDiv);
                    span.appendChild(toggle);
                    headerToggles.push(li);
                }
            }
            li.appendChild(span);

            const currentParent = stack[stack.length - 1];
            currentParent.ol.appendChild(li);
        }

        const onThisPage = document.createElement('div');
        onThisPage.classList.add('on-this-page');
        onThisPage.append(stack[0].ol);
        const activeItemSpan = activeSection.parentElement;
        activeItemSpan.after(onThisPage);
    });

    document.addEventListener('DOMContentLoaded', reloadCurrentHeader);
    document.addEventListener('scroll', reloadCurrentHeader, { passive: true });
})();

