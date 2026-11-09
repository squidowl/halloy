// Extracted from sys-locale but adjusted for time usage

pub(crate) fn get_time_locale() -> Option<String> {
    // As per The Open Group specification https://pubs.opengroup.org/onlinepubs/7908799/xbd/envvar.html
    // precedence is LC_ALL → LC_TIME → LANG
    ["LC_ALL", "LC_TIME", "LANG"]
        .into_iter()
        .find_map(|var| std::env::var(var).ok().filter(|v| !v.is_empty()))
        .map(|v| super::posix_to_bcp47(&v))
        .filter(|v| !v.is_empty() && v != "C" && v != "POSIX")
}
