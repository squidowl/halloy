use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use core_foundation_sys::locale::{CFLocaleCopyCurrent, CFLocaleGetIdentifier};

pub(crate) fn get_locale() -> Option<String> {
    let identifier = unsafe {
        CFString::wrap_under_get_rule(CFLocaleGetIdentifier(
            CFLocaleCopyCurrent(),
        ))
    };

    let locale = super::posix_to_bcp47(&identifier.to_string());
    if locale.is_empty() {
        None
    } else {
        Some(locale)
    }
}
