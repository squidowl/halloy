use iced::Font;

pub const REGULAR: Font = Font::External {
    name: "Iosevka Term Regular",
    bytes: include_bytes!("../fonts/iosevka-term-regular.ttf"),
};

pub const BOLD: Font = Font::External {
    name: "Iosevka Term Bold",
    bytes: include_bytes!("../fonts/iosevka-term-bold.ttf"),
};

pub const ITALIC: Font = Font::External {
    name: "Iosevka Term Italic",
    bytes: include_bytes!("../fonts/iosevka-term-italic.ttf"),
};
