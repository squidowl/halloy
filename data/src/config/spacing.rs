use serde::Deserialize;

#[derive(Debug, Clone, Default, Copy, Deserialize)]
#[serde(default)]
pub struct Spacing {
    pub buffer: Buffer,
    pub pane: Pane,
    pub sidebar: Sidebar,
    pub context_menu: ContextMenu,
}

#[derive(Debug, Clone, Default, Copy, Deserialize)]
#[serde(default)]
pub struct Buffer {
    pub line_spacing: u32,
}

#[derive(Debug, Clone, Default, Copy, Deserialize)]
#[serde(default)]
pub struct Pane {
    pub gap: Gap,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(default)]
pub struct Gap {
    pub inner: u32,
    pub outer: u16,
}

impl Default for Gap {
    fn default() -> Self {
        Self { inner: 4, outer: 8 }
    }
}

#[derive(Debug, Clone, Default, Copy, Deserialize)]
#[serde(default)]
pub struct Sidebar {
    pub padding: SidebarPadding,
    pub spacing: SidebarSpacing,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(default)]
pub struct SidebarPadding {
    pub buffer: [u16; 2],
}

impl Default for SidebarPadding {
    fn default() -> Self {
        Self { buffer: [5, 5] }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(default)]
pub struct SidebarSpacing {
    pub server: u32,
}

impl Default for SidebarSpacing {
    fn default() -> Self {
        Self { server: 12 }
    }
}

#[derive(Debug, Clone, Default, Copy, Deserialize)]
#[serde(default)]
pub struct ContextMenu {
    pub padding: ContextMenuPadding,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(default)]
pub struct ContextMenuPadding {
    pub entry: [u16; 2],
}

impl Default for ContextMenuPadding {
    fn default() -> Self {
        Self { entry: [5, 5] }
    }
}
