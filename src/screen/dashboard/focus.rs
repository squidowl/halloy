use data::{Buffer, buffer};

pub(super) fn replacement_buffer_after_close(
    closed: Option<&Buffer>,
    parent: Option<&Buffer>,
    previous: Option<&Buffer>,
    available: Vec<buffer::Upstream>,
) -> Option<Buffer> {
    // Preserve explicit user intent first. Network fallbacks only apply when no
    // recorded parent or previous focused buffer can take over.
    parent
        .filter(|parent| Some(*parent) != closed)
        .or_else(|| previous.filter(|previous| Some(*previous) != closed))
        .cloned()
        .or_else(|| same_network_fallback(closed, available))
}

fn same_network_fallback(
    closed: Option<&Buffer>,
    available: Vec<buffer::Upstream>,
) -> Option<Buffer> {
    let closed_upstream = closed.and_then(Buffer::upstream)?;
    let server = closed_upstream.server();

    available
        .iter()
        .filter(|candidate| *candidate != closed_upstream)
        .find(|candidate| {
            candidate.server() == server
                && !matches!(candidate, buffer::Upstream::Server(_))
        })
        .or_else(|| {
            available.iter().find(|candidate| {
                matches!(candidate, buffer::Upstream::Server(candidate_server) if candidate_server == server)
            })
        })
        .cloned()
        .map(Buffer::Upstream)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use data::{Server, isupport, target};

    use super::*;

    fn server(name: &str) -> Server {
        Server {
            name: Arc::from(name),
            network: None,
        }
    }

    fn channel(name: &str) -> target::Channel {
        target::Channel::from_str(name, &['#'], isupport::CaseMap::default())
    }

    fn query(name: &str) -> target::Query {
        target::Query::parse(name, &['#'], &[], isupport::CaseMap::default())
            .expect("valid query")
    }

    fn upstream_channel(server: &Server, name: &str) -> buffer::Upstream {
        buffer::Upstream::Channel(server.clone(), channel(name))
    }

    fn upstream_query(server: &Server, name: &str) -> buffer::Upstream {
        buffer::Upstream::Query(server.clone(), query(name))
    }

    #[test]
    fn replacement_prefers_parent_buffer() {
        let server = server("libera");
        let closed = Buffer::Upstream(upstream_query(&server, "alice"));
        let parent = Buffer::Upstream(upstream_channel(&server, "#rust"));
        let previous = Buffer::Upstream(upstream_channel(&server, "#halloy"));

        let replacement = replacement_buffer_after_close(
            Some(&closed),
            Some(&parent),
            Some(&previous),
            vec![upstream_channel(&server, "#linux")],
        );

        assert_eq!(replacement, Some(parent));
    }

    #[test]
    fn replacement_uses_previous_when_parent_is_closed_buffer() {
        let server = server("libera");
        let closed = Buffer::Upstream(upstream_query(&server, "alice"));
        let previous = Buffer::Upstream(upstream_channel(&server, "#halloy"));

        let replacement = replacement_buffer_after_close(
            Some(&closed),
            Some(&closed),
            Some(&previous),
            vec![upstream_channel(&server, "#linux")],
        );

        assert_eq!(replacement, Some(previous));
    }

    #[test]
    fn replacement_falls_back_to_same_network_buffer_before_server() {
        let server = server("libera");
        let closed = Buffer::Upstream(upstream_query(&server, "alice"));
        let fallback = upstream_channel(&server, "#rust");

        let replacement = replacement_buffer_after_close(
            Some(&closed),
            None,
            None,
            vec![buffer::Upstream::Server(server.clone()), fallback.clone()],
        );

        assert_eq!(replacement, Some(Buffer::Upstream(fallback)));
    }

    #[test]
    fn replacement_falls_back_to_same_network_server() {
        let server = server("libera");
        let closed = Buffer::Upstream(upstream_query(&server, "alice"));

        let replacement = replacement_buffer_after_close(
            Some(&closed),
            None,
            None,
            vec![buffer::Upstream::Server(server.clone())],
        );

        assert_eq!(
            replacement,
            Some(Buffer::Upstream(buffer::Upstream::Server(server)))
        );
    }

    #[test]
    fn replacement_does_not_cross_networks() {
        let libera = server("libera");
        let oftc = server("oftc");
        let closed = Buffer::Upstream(upstream_query(&libera, "alice"));

        let replacement = replacement_buffer_after_close(
            Some(&closed),
            None,
            None,
            vec![upstream_channel(&oftc, "#debian")],
        );

        assert_eq!(replacement, None);
    }
}
