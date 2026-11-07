// Extracted from sys-locale but adjusted for time usage

pub(crate) fn get_time_locale() -> Option<String> {
    ["LC_ALL", "LC_TIME", "LANG"]
        .into_iter()
        .find_map(|var| std::env::var(var).ok().filter(|v| !v.is_empty()))
        .map(|v| super::posix_to_bcp47(&v))
        .filter(|v| !v.is_empty() && v != "C" && v != "POSIX")
}
