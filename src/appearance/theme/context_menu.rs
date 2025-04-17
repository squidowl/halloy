use super::Theme;
use crate::widget::context_menu::{Catalog, Style, StyleFn};

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(super::container::tooltip)
    }

    fn style(&self, class: &Self::Class<'_>) -> Style {
        class(self)
    }
}
