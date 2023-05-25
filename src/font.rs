use iced::{
    font::{self, Error},
    Command, Font,
};

pub const MONO: Font = Font {
    monospaced: true,
    ..Font::with_name("Iosevka Term")
};

pub const MONO_BOLD: Font = Font {
    monospaced: true,
    weight: font::Weight::Bold,
    ..Font::with_name("Iosevka Term")
};

pub const ICON: Font = Font {
    monospaced: true,
    ..Font::with_name("bootstrap-icons")
};

pub fn load() -> Command<Result<(), Error>> {
    Command::batch(vec![
        font::load(include_bytes!("../fonts/iosevka-term-regular.ttf").as_slice()),
        font::load(include_bytes!("../fonts/iosevka-term-bold.ttf").as_slice()),
        font::load(include_bytes!("../fonts/iosevka-term-italic.ttf").as_slice()),
        font::load(include_bytes!("../fonts/icons.ttf").as_slice()),
    ])
}
