use iced::widget::text_editor::{self, Content, Cursor};

#[derive(Debug, Clone)]
struct Snapshot {
    text: String,
    cursor: Cursor,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum EditKind {
    Insert,
    Delete,
}

fn edit_kind(edit: &text_editor::Edit) -> Option<EditKind> {
    match edit {
        text_editor::Edit::Insert(c) if !c.is_whitespace() => {
            Some(EditKind::Insert)
        }
        text_editor::Edit::Backspace | text_editor::Edit::Delete => {
            Some(EditKind::Delete)
        }
        _ => None,
    }
}

#[derive(Debug, Default, Clone)]
pub struct History {
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    coalescing: Option<EditKind>,
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }

    fn snapshot(content: &Content) -> Snapshot {
        Snapshot {
            text: content.text(),
            cursor: content.cursor(),
        }
    }

    pub fn track(&mut self, content: &Content, action: &text_editor::Action) {
        let text_editor::Action::Edit(edit) = action else {
            // Any non-edit action ends the current undo group.
            self.coalescing = None;
            return;
        };

        let kind = edit_kind(edit);

        if kind.is_none() || kind != self.coalescing {
            self.undo.push(Self::snapshot(content));
            self.redo.clear();
        }

        self.coalescing = kind;
    }

    pub fn checkpoint(&mut self, content: &Content) {
        self.undo.push(Self::snapshot(content));
        self.redo.clear();
        self.coalescing = None;
    }

    pub fn undo(&mut self, content: &mut Content) -> bool {
        let Some(snapshot) = self.undo.pop() else {
            return false;
        };

        self.redo.push(Self::snapshot(content));
        self.restore(content, snapshot);

        true
    }

    pub fn redo(&mut self, content: &mut Content) -> bool {
        let Some(snapshot) = self.redo.pop() else {
            return false;
        };

        self.undo.push(Self::snapshot(content));
        self.restore(content, snapshot);

        true
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.coalescing = None;
    }

    fn restore(&mut self, content: &mut Content, snapshot: Snapshot) {
        *content = Content::with_text(&snapshot.text);
        content.move_to(snapshot.cursor);
        self.coalescing = None;
    }
}
