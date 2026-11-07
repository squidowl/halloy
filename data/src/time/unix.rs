// Extracted from sys-locale but adjusted for time usage

fn posix_to_bcp47(locale: &str) -> String {
    locale
        .chars()
        .take_while(|&c| c != '.' && c != '@')
        .map(|c| if c == '_' { '-' } else { c })
        .collect()
}

pub(crate) fn get_time_locales() -> impl Iterator<Item = String> {
    let mut locales = Vec::new();

    // LANGUAGE contains one or multiple locales separated by colon (':')
    if let Some(val) =
        std::env::var("LANGUAGE").ok().filter(|val| !val.is_empty())
    {
        for part in val.split(':') {
            let locale = posix_to_bcp47(part);
            if !locales.contains(&locale) {
                locales.push(locale);
            }
        }
    }

    // LC_ALL, LC_TIME and LANG contain one locale
    for variable in ["LC_ALL", "LC_TIME", "LANG"] {
        if let Some(val) =
            std::env::var(variable).ok().filter(|val| !val.is_empty())
        {
            let locale = posix_to_bcp47(&val);
            if !locales.contains(&locale) {
                locales.push(locale);
            }
        }
    }

    locales.into_iter()
}
