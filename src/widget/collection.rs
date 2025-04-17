use iced::Element;
use iced::widget::{Column, Row};

pub trait Collection<'a, Message, Theme>: Sized {
    fn push(self, element: impl Into<Element<'a, Message, Theme>>) -> Self;

    fn push_maybe(
        self,
        element: Option<impl Into<Element<'a, Message, Theme>>>,
    ) -> Self {
        match element {
            Some(element) => self.push(element),
            None => self,
        }
    }
}

impl<'a, Message, Theme> Collection<'a, Message, Theme>
    for Column<'a, Message, Theme>
{
    fn push(self, element: impl Into<Element<'a, Message, Theme>>) -> Self {
        Self::push(self, element)
    }
}

impl<'a, Message, Theme> Collection<'a, Message, Theme>
    for Row<'a, Message, Theme>
{
    fn push(self, element: impl Into<Element<'a, Message, Theme>>) -> Self {
        Self::push(self, element)
    }
}
