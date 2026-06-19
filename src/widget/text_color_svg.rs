use iced::advanced::svg::Svg;
use iced::advanced::widget::Tree;
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, renderer, svg};
use iced::widget::text::{Catalog, Style, StyleFn};
use iced::{
    ContentFit, Element, Event, Length, Point, Radians, Rectangle, Rotation,
    Size, Vector, mouse,
};

pub fn text_color_svg<'a, Theme>(
    handle: impl Into<svg::Handle>,
) -> TextColorSvg<'a, Theme>
where
    Theme: Catalog,
{
    TextColorSvg::new(handle)
}

// Styled using a text::Style, falling back to the default text color when no
// style is specified.  For use in interface elements, where an SVG is expected
// to be colored and being potentially-colorable by the default text color is
// useful (e.g. if an SVG is the content of a button).
pub struct TextColorSvg<'a, Theme>
where
    Theme: Catalog,
{
    svg: Svg,
    width: Length,
    height: Length,
    content_fit: ContentFit,
    class: Theme::Class<'a>,
}

impl<'a, Theme> TextColorSvg<'a, Theme>
where
    Theme: Catalog,
{
    pub fn new(handle: impl Into<svg::Handle>) -> Self {
        TextColorSvg {
            svg: Svg::new(handle),
            width: Length::Shrink,
            height: Length::Shrink,
            content_fit: ContentFit::Contain,
            class: Theme::default(),
        }
    }

    #[must_use]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    #[must_use]
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    #[must_use]
    pub fn content_fit(self, content_fit: ContentFit) -> Self {
        Self {
            content_fit,
            ..self
        }
    }

    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    #[must_use]
    pub fn rotation(mut self, rotation: impl Into<Radians>) -> Self {
        self.svg = self.svg.rotation(rotation);
        self
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for TextColorSvg<'_, Theme>
where
    Renderer: svg::Renderer,
    Theme: Catalog,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        // The raw w/h of the underlying image
        let Size { width, height } = renderer.measure_svg(&self.svg.handle);
        let image_size = Size::new(width as f32, height as f32);

        let rotation = Rotation::from(self.svg.rotation);

        // The rotated size of the svg
        let rotated_size = rotation.apply(image_size);

        // The size to be available to the widget prior to `Shrink`ing
        let raw_size = limits.resolve(self.width, self.height, rotated_size);

        // The uncropped size of the image when fit to the bounds above
        let full_size = self.content_fit.fit(rotated_size, raw_size);

        // Shrink the widget to fit the resized image, if requested
        let final_size = Size {
            width: match self.width {
                Length::Shrink => f32::min(raw_size.width, full_size.width),
                _ => raw_size.width,
            },
            height: match self.height {
                Length::Shrink => f32::min(raw_size.height, full_size.height),
                _ => raw_size.height,
            },
        };

        layout::Node::new(final_size)
    }

    fn update(
        &mut self,
        _state: &mut Tree,
        _event: &Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        _shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
    }

    fn draw(
        &self,
        _state: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let Size { width, height } = renderer.measure_svg(&self.svg.handle);
        let image_size = Size::new(width as f32, height as f32);
        let rotation = Rotation::from(self.svg.rotation);
        let rotated_size = rotation.apply(image_size);

        let bounds = layout.bounds();
        let adjusted_fit = self.content_fit.fit(rotated_size, bounds.size());
        let scale = Vector::new(
            adjusted_fit.width / rotated_size.width,
            adjusted_fit.height / rotated_size.height,
        );

        let final_size = image_size * scale;

        let position = match self.content_fit {
            ContentFit::None => Point::new(
                bounds.x + (rotated_size.width - adjusted_fit.width) / 2.0,
                bounds.y + (rotated_size.height - adjusted_fit.height) / 2.0,
            ),
            _ => Point::new(
                bounds.center_x() - final_size.width / 2.0,
                bounds.center_y() - final_size.height / 2.0,
            ),
        };

        let drawing_bounds = Rectangle::new(position, final_size);

        let color = theme.style(&self.class).color.unwrap_or(style.text_color);

        renderer.draw_svg(
            svg::Svg {
                handle: self.svg.handle.clone(),
                color: Some(color),
                rotation: self.svg.rotation,
                opacity: color.a,
            },
            drawing_bounds,
            bounds,
        );
    }
}

impl<'a, Message, Theme, Renderer> From<TextColorSvg<'a, Theme>>
    for Element<'a, Message, Theme, Renderer>
where
    Renderer: svg::Renderer + 'a,
    Theme: Catalog + 'a,
{
    fn from(
        icon: TextColorSvg<'a, Theme>,
    ) -> Element<'a, Message, Theme, Renderer> {
        Element::new(icon)
    }
}
