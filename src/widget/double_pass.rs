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
        &mut self,
        _state: &mut (),
        second_pass: &mut iced::Element<'a, Message, Theme, Renderer>,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> layout::Node {
        let mut first_pass_tree = widget::Tree::new(&self.first_pass);
        let layout = self.first_pass.as_widget_mut().layout(
            &mut first_pass_tree,
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

        second_pass
            .as_widget_mut()
            .layout(tree, renderer, &new_limits)
    }
}

pub fn horizontal_expansion() -> f32 {
    1.0
}
