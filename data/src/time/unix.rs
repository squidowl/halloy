// Extracted from sys-locale but adjusted for time usage

fn posix_to_bcp47(locale: &str) -> String {
    locale
        .chars()
        .take_while(|&c| c != '.' && c != '@')
        .map(|c| if c == '_' { '-' } else { c })
        .collect()
}

pub(crate) fn get_time_locale() -> Option<String> {
    ["LC_ALL", "LC_TIME", "LANG"]
        .into_iter()
        .find_map(|var| std::env::var(var).ok().filter(|v| !v.is_empty()))
        .map(|v| posix_to_bcp47(&v))
        .filter(|v| !v.is_empty() && v != "C" && v != "POSIX")
}
