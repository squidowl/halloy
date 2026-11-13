#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CaseMap {
    ASCII,
    RFC1459,
    RFC1459_STRICT,
    #[default]
    RFC7613,
}

impl CaseMap {
    pub fn normalize(&self, from_str: &str) -> String {
        match self {
            CaseMap::ASCII => from_str.to_ascii_lowercase(),
            CaseMap::RFC1459 => from_str
                .chars()
                .map(|c| match c {
                    'A' => 'a',
                    'B' => 'b',
                    'C' => 'c',
                    'D' => 'd',
                    'E' => 'e',
                    'F' => 'f',
                    'G' => 'g',
                    'H' => 'h',
                    'I' => 'i',
                    'J' => 'j',
                    'K' => 'k',
                    'L' => 'l',
                    'M' => 'm',
                    'N' => 'n',
                    'O' => 'o',
                    'P' => 'p',
                    'Q' => 'q',
                    'R' => 'r',
                    'S' => 's',
                    'T' => 't',
                    'U' => 'u',
                    'V' => 'v',
                    'W' => 'w',
                    'X' => 'x',
                    'Y' => 'y',
                    'Z' => 'z',
                    '[' => '{',
                    ']' => '}',
                    '\\' => '|',
                    '~' => '^',
                    _ => c,
                })
                .collect(),
            CaseMap::RFC1459_STRICT => from_str
                .chars()
                .map(|c| match c {
                    'A' => 'a',
                    'B' => 'b',
                    'C' => 'c',
                    'D' => 'd',
                    'E' => 'e',
                    'F' => 'f',
                    'G' => 'g',
                    'H' => 'h',
                    'I' => 'i',
                    'J' => 'j',
                    'K' => 'k',
                    'L' => 'l',
                    'M' => 'm',
                    'N' => 'n',
                    'O' => 'o',
                    'P' => 'p',
                    'Q' => 'q',
                    'R' => 'r',
                    'S' => 's',
                    'T' => 't',
                    'U' => 'u',
                    'V' => 'v',
                    'W' => 'w',
                    'X' => 'x',
                    'Y' => 'y',
                    'Z' => 'z',
                    '[' => '{',
                    ']' => '}',
                    '\\' => '|',
                    _ => c,
                })
                .collect(),
            CaseMap::RFC7613 => from_str.to_lowercase(),
        }
    }
}
