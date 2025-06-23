//! A widget that uses a two pass layout.
//!
//! Layout from first pass is used as the limits for the second pass

use iced::advanced::{self, layout, widget};
use iced::{Element, Size};

use super::decorate;

/// Layout from first pass is used as the limits for the second pass
pub fn double_pass<'a, Message, Theme, Renderer>(
    first_pass: impl Into<Element<'a, Message, Theme, Renderer>>,
    second_pass: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: advanced::Renderer + 'a,
{
    decorate(second_pass)
        .layout(Layout {
            first_pass: first_pass.into(),
        })
        .into()
}

struct Layout<'a, Message, Theme, Renderer> {
    first_pass: Element<'a, Message, Theme, Renderer>,
}

impl<'a, Message, Theme, Renderer>
    decorate::Layout<'a, Message, Theme, Renderer, ()>
    for Layout<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: advanced::Renderer + 'a,
{
    fn layout(
        &self,
        _state: &mut (),
        second_pass: &iced::Element<'a, Message, Theme, Renderer>,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> layout::Node {
        let layout = self.first_pass.as_widget().layout(
            &mut widget::Tree::new(&self.first_pass),
            renderer,
            limits,
        );

        let new_limits = layout::Limits::new(
            Size::ZERO,
            layout
                .size()
                // eliminate float precision issues if second pass
                // is fill
                .expand(Size::new(horizontal_expansion(), 1.0)),
        );

        second_pass.as_widget().layout(tree, renderer, &new_limits)
    }
}

pub fn horizontal_expansion() -> f32 {
    1.0
}
