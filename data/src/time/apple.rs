use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use core_foundation_sys::locale::{CFLocaleCopyCurrent, CFLocaleGetIdentifier};

pub(crate) fn get_locale() -> Option<String> {
    // Use locale for date formatting, as recommended by the Date Formatters
    // section in Apple's Data Formatting Guide
    // https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/DataFormatting/Articles/dfDateFormatting10_4.html
    let identifier = unsafe {
        CFString::wrap_under_get_rule(CFLocaleGetIdentifier(
            CFLocaleCopyCurrent(),
        ))
    };

    // From testing, CFLocaleGetIdentifier may return a POSIX-style identifier.
    let locale = super::posix_to_bcp47(&identifier.to_string());
    if locale.is_empty() {
        None
    } else {
        Some(locale)
    }
}
