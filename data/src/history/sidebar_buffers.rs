use std::collections::{HashMap, HashSet};

use crate::target::{self, Target};
use crate::{Server, client, history};

#[derive(Debug, Default)]
struct Buffers {
    channels: HashSet<target::Channel>,
    queries: HashSet<target::Query>,
}

#[derive(Debug, Default)]
pub struct SidebarBuffers {
    buffers: HashMap<Server, Buffers>,
    disconnected: HashMap<Server, HashSet<target::Channel>>,
    closing: HashMap<Server, HashSet<target::Channel>>,
    removed: HashSet<Server>,
    remember_buffers: bool,
    changed: bool,
    dirty: bool,
    exited: bool,
}

impl SidebarBuffers {
    pub fn restore(&mut self, server: Server, target: Target) {
        let buffers = self.buffers.entry(server.clone()).or_default();

        match target {
            Target::Channel(channel) => {
                buffers.channels.insert(channel.clone());

                self.disconnected.entry(server).or_default().insert(channel);
            }
            Target::Query(query) => {
                buffers.queries.insert(query);
            }
        }
    }

    pub fn disconnected_channels(
        &self,
        server: &Server,
    ) -> impl Iterator<Item = &target::Channel> {
        self.disconnected.get(server).into_iter().flatten()
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn mark_channel_disconnected(
        &mut self,
        server: Server,
        channel: target::Channel,
    ) {
        self.dirty = true;

        if let Some(channels) = self.closing.get_mut(&server)
            && channels.remove(&channel)
        {
            if channels.is_empty() {
                self.closing.remove(&server);
            }

            return;
        }

        self.disconnected.entry(server).or_default().insert(channel);
    }

    pub fn mark_channel_joined(
        &mut self,
        server: &Server,
        channel: &target::Channel,
    ) {
        self.dirty = true;
        self.unmark_disconnected(server, channel);
    }

    pub fn forget_closed_channel(
        &mut self,
        kind: &history::Kind,
        clients: &client::Map,
    ) {
        if let history::Kind::Channel(server, channel) = kind {
            self.dirty = true;
            self.unmark_disconnected(server, channel);

            if let Some(buffers) = self.buffers.get_mut(server) {
                self.changed |= buffers.channels.remove(channel);
            }

            // only joined channels have a PART in flight
            if clients.contains_channel(server, channel) {
                self.closing
                    .entry(server.clone())
                    .or_default()
                    .insert(channel.clone());
            }
        }
    }

    pub fn mark_server_channels_disconnected(
        &mut self,
        server: &Server,
        clients: &client::Map,
    ) {
        self.dirty = true;

        let closing = self.closing.remove(server);

        let channels = clients
            .get_channels(server)
            .filter(|channel| {
                !closing
                    .as_ref()
                    .is_some_and(|closing| closing.contains(*channel))
            })
            .cloned()
            .collect::<Vec<_>>();

        if !channels.is_empty() {
            self.disconnected
                .entry(server.clone())
                .or_default()
                .extend(channels);
        }
    }

    pub fn forget_server(&mut self, server: &Server) {
        self.dirty = true;

        if self.buffers.remove(server).is_some() {
            self.changed = true;
        }

        self.disconnected.remove(server);
        self.closing.remove(server);
        self.removed.insert(server.clone());
    }

    pub fn forget_unlisted_bouncer_networks(
        &mut self,
        parent: &Server,
        listed: &[Server],
    ) {
        let unlisted = self
            .buffers
            .keys()
            .chain(self.disconnected.keys())
            .filter(|server| {
                server.parent().as_ref() == Some(parent)
                    && !listed.contains(server)
            })
            .cloned()
            .collect::<Vec<_>>();

        for server in unlisted {
            self.forget_server(&server);
        }
    }

    fn unmark_disconnected(
        &mut self,
        server: &Server,
        channel: &target::Channel,
    ) {
        if let Some(channels) = self.disconnected.get_mut(server) {
            channels.remove(channel);

            if channels.is_empty() {
                self.disconnected.remove(server);
            }
        }
    }

    /// true if saved buffers changed
    pub fn update_saved<'a>(
        &mut self,
        queries: impl Iterator<Item = (&'a Server, &'a target::Query)> + Clone,
        clients: &client::Map,
        remember_buffers: bool,
    ) -> bool {
        if self.exited {
            return false;
        }

        if remember_buffers != self.remember_buffers {
            self.remember_buffers = remember_buffers;
            self.dirty = true;
        }

        if !remember_buffers {
            let cleared = self.buffers.values().any(|buffers| {
                !buffers.channels.is_empty() || !buffers.queries.is_empty()
            });

            self.buffers.clear();
            self.changed = false;
            self.dirty = false;

            return cleared;
        }

        // removed server reconnecting gets its queries saved again
        if self
            .removed
            .iter()
            .any(|server| clients.client(server).is_some())
        {
            self.dirty = true;
        }

        let mut changed = std::mem::take(&mut self.changed);

        if !std::mem::take(&mut self.dirty) {
            return changed;
        }

        for server in clients.connected_servers() {
            self.removed.remove(server);

            let closing = self.closing.get(server);

            // left channels stay saved, buffer is still open
            for channel in clients.get_channels(server) {
                // closed channels stay unsaved while part is in flight
                if closing.is_some_and(|closing| closing.contains(channel)) {
                    continue;
                }

                if !self
                    .buffers
                    .get(server)
                    .is_some_and(|buffers| buffers.channels.contains(channel))
                {
                    self.buffers
                        .entry(server.clone())
                        .or_default()
                        .channels
                        .insert(channel.clone());
                    changed = true;
                }
            }
        }

        let removed = &self.removed;
        let queries =
            queries.filter(move |(server, _)| !removed.contains(*server));

        if !self.queries_unchanged(queries.clone()) {
            for buffers in self.buffers.values_mut() {
                buffers.queries.clear();
            }

            for (server, query) in queries {
                self.buffers
                    .entry(server.clone())
                    .or_default()
                    .queries
                    .insert(query.clone());
            }

            changed = true;
        }

        self.buffers.retain(|_, buffers| {
            !buffers.channels.is_empty() || !buffers.queries.is_empty()
        });

        changed
    }

    fn queries_unchanged<'a>(
        &self,
        mut queries: impl Iterator<Item = (&'a Server, &'a target::Query)>,
    ) -> bool {
        let mut count = 0;

        queries.all(|(server, query)| {
            count += 1;

            self.buffers
                .get(server)
                .is_some_and(|buffers| buffers.queries.contains(query))
        }) && count
            == self
                .buffers
                .values()
                .map(|buffers| buffers.queries.len())
                .sum::<usize>()
    }

    pub fn exit(&mut self) {
        self.exited = true;
    }

    pub fn to_vec(&self) -> Vec<(Server, Target)> {
        self.buffers
            .iter()
            .flat_map(|(server, buffers)| {
                buffers
                    .channels
                    .iter()
                    .cloned()
                    .map(Target::Channel)
                    .chain(buffers.queries.iter().cloned().map(Target::Query))
                    .map(|target| (server.clone(), target))
            })
            .collect()
    }
}
