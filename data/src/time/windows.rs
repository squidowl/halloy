use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;
use windows_sys::Win32::System::SystemServices::LOCALE_NAME_MAX_LENGTH;

pub(crate) fn get_locale() -> Option<String> {
    let mut buffer = [0; LOCALE_NAME_MAX_LENGTH as usize];

    let len = unsafe {
        GetUserDefaultLocaleName(buffer.as_mut_ptr(), buffer.len() as i32)
    };

    // The length includes the null terminator, and is zero if no locale is set.
    if len > 0 {
        String::from_utf16(&buffer[..len as usize])
            .ok()
            .map(|locale| locale.trim_matches('\0').to_string())
            .filter(|locale| !locale.is_empty())
    } else {
        None
    }
}
