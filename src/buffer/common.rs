use data::buffer::Upstream;
use data::user::Nick;
use data::{client, command};

/// Builds display entries for `/common` from Halloy's live client state.
///
/// Used by the input view command dispatcher. `buffer` supplies the current
/// context and must be a channel, `clients` supplies already-known in-memory
/// membership state, and `command` supplies the selected scope. Returns sorted
/// `/common` entries or a local error message. Produces no network traffic,
/// filesystem access, or persistent output.
pub fn entries(
    buffer: &Upstream,
    clients: &client::Map,
    command: command::common::Command,
) -> Result<Vec<command::common::Entry>, String> {
    // `/common` is defined around a current channel. Server and query buffers
    // do not provide the anchor channel needed to decide which users to check.
    let Upstream::Channel(current_server, current_channel) = buffer else {
        return Err("Common channels are available only in channel buffers."
            .to_string());
    };

    // Candidate nicks are copied out of live channel state before any broader
    // scan. This keeps the later network/global passes free of long borrows
    // against the client map.
    let nicks = candidate_nicks(buffer, clients);

    // Scope selection is deliberately explicit. Global is the default command
    // behavior, but network remains useful when the user wants current-server
    // output without network prefixes.
    let entries = match command.scope {
        command::common::Scope::Network => {
            network_entries(clients, current_server, current_channel, &nicks)
        }
        command::common::Scope::Global => {
            global_entries(clients, current_server, current_channel, &nicks)
        }
    };

    // The data command module owns sorting, deduplication, and omission of
    // users with no remaining shared channels, so the UI layer cannot drift.
    Ok(command::common::summarize(entries))
}

/// Extracts candidate nicks from the spawning channel.
///
/// Used by `entries`. `buffer` identifies the current channel and `clients`
/// provides its in-memory user list. Returns owned nicks excluding the local
/// user. Produces no output or side effects.
fn candidate_nicks(buffer: &Upstream, clients: &client::Map) -> Vec<Nick> {
    // The caller has already checked that the buffer is a channel. Matching
    // here keeps this helper total and avoids panicking if it is reused later.
    let Upstream::Channel(server, channel) = buffer else {
        return Vec::new();
    };

    // If Halloy has no current user list for the channel, there is no safe
    // local data to inspect. Returning an empty list keeps `/common` local-only.
    let Some(users) = clients.get_channel_users(server, channel) else {
        return Vec::new();
    };

    // The local user is excluded before any scope-specific scan, so both
    // network and global modes share the same privacy behavior.
    let own_nick = clients.nickname(server);

    users
        .iter()
        .filter_map(|user| {
            // Skip the local user's own nick. It would otherwise commonly
            // appear in every joined channel and dominate the result list.
            if own_nick.is_some_and(|own_nick| user.nickname() == own_nick) {
                None
            } else {
                // Store an owned normalized nick so later lookups can use
                // Halloy's server-aware nick comparison without borrowing the
                // current channel user list.
                Some(Nick::from(user.nickname()))
            }
        })
        .collect()
}

/// Builds `/common scope=network` entries.
///
/// Used by `entries`. `clients` supplies local membership state,
/// `current_server` and `current_channel` identify the anchor, and `nicks` are
/// candidates from that channel. Returns unnormalized entries scoped to the
/// current server. Produces no output or side effects.
fn network_entries(
    clients: &client::Map,
    current_server: &data::Server,
    current_channel: &data::target::Channel,
    nicks: &[Nick],
) -> Vec<command::common::Entry> {
    nicks
        .iter()
        .map(|nick| {
            // Network scope checks only the current server and omits the
            // channel that spawned the command from the displayed overlap.
            let channels = clients
                .get_user_channels(current_server, nick.as_nickref())
                .into_iter()
                .filter(|shared| shared != current_channel)
                .map(|shared| shared.to_string())
                .collect();

            command::common::Entry {
                nick: nick.to_string(),
                channels,
            }
        })
        .collect()
}

/// Builds `/common scope=global` entries.
///
/// Used by `entries`. `clients` supplies all connected server state,
/// `current_server` and `current_channel` identify the anchor to omit, and
/// `nicks` are candidates from that channel. Returns unnormalized entries with
/// `network/nick` labels. Produces no network traffic, output, or side effects.
fn global_entries(
    clients: &client::Map,
    current_server: &data::Server,
    current_channel: &data::target::Channel,
    nicks: &[Nick],
) -> Vec<command::common::Entry> {
    let mut entries = Vec::new();

    // Global scope still reads only Halloy's in-memory state. It walks the
    // connected server set and never sends WHOIS, WHO, or any other IRC query.
    for server in clients.connected_servers() {
        // Each candidate nick from the spawning channel is checked on each
        // connected network so output can show `network/nick` explicitly.
        for nick in nicks {
            let channels = clients
                .get_user_channels(server, nick.as_nickref())
                .into_iter()
                .filter(|shared| {
                    // The spawning channel is context, not a result. Other
                    // channels with the same name on other networks remain
                    // valid because their server differs.
                    server != current_server || shared != current_channel
                })
                .map(|shared| shared.to_string())
                .collect();

            entries.push(command::common::Entry {
                nick: format!("{server}/{nick}"),
                channels,
            });
        }
    }

    entries
}
