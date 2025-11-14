# Matches

Highlight based on matches

- [Matches](#matches)
  - [Example](#example)
  - [Configuration](#configuration)
    - [words](#words)
    - [case\_insensitive](#case_insensitive)
    - [regex](#regex)
    - [exclude](#exclude)
    - [include](#include)
    - [sound](#sound)

## Example

```toml
# Highlight on 'boat' and 'car' in any channel.
[[highlights.match]]
words = ["boat", "car"]
case_insensitive = true
sound = "bonk"

# Highlight when regex matches in any channel except #noisy-channel.
[[highlights.match]]
regex = '''(?i)\bcasper\b'''
exclude = ["#noisy-channel"]
```

## Configuration

### words

You can set words to be highlighted when they are written.

Example shows word matches, which will trigger on `"word1"`, `"word2"` or `"word3"` in any channel.

```toml
# Type: array of strings
# Values: array of any strings
# Default: []

[[highlights.match]]
words = ["word1", "word2", "word3"]
```

### case_insensitive

This option is only available when using `words` as the match type.
You can choose whether or not to trigger regardless of case.

```toml
# Type: boolean
# Values: true, false
# Default: false

[[highlights.match]]
words = ["word1", "word2", "word3"]
case_insensitive = true
```

### regex

Match based on regex.

<div class="warning">

Use toml multi-line literal strings `'''\bfoo'd\b'''` when writing a regex. This allows you to write write the regex without
escaping. You can also use a literal string `'\bfoo\b'`, but then you can't use `'` inside the string.

Without literal strings, you'd have to write the above as `"\\bfoo'd\\b"`

</div>

Example shows a regex that matches the word "casper", regardless of case and only when it appears as a whole word in any channel.

```toml
# Type: string
# Values: any string
# Default: not set

[[highlights.match]]
regex = '''(?i)\bcasper\b'''
```

### exclude

[Exclusion conditions](/configuration/conditions.md) in which you won't be
highlighted. Inclusion conditions will take precedence over exclusion
conditions. You can also exclude all conditions by setting to `"all"` or `"*"`.

Example shows a regex match which will be excluded in `#noisy-channel`.

```toml
# Type: inclusion/exclusion conditions
# Values: any inclusion/exclusion conditions
# Default: not set

[[highlights.match]]
regex = '''(?i)\bcasper\b'''
exclude = { channels = ["#noisy-channel"] }
```

### include

[Inclusion conditions](/configuration/conditions.md) in which you will be
highlighted. Highlights are enabled in all conditions unless explicitly
excluded, so this setting is only relevant when combined with the `exclude`
setting.

Example shows a words match which will only highlight in `#halloy`.

```toml
# Type: inclusion/exclusion conditions
# Values: any inclusion/exclusion conditions
# Default: not set

[[highlights.match]]
words = ["word1", "word2", "word3"]
exclude = "*"
include = { channels = ["#halloy"] }
```

### sound

Sound to play when notifying for a highlight. If not specified then the sound
specified for highlight notifications will be used. Supports both built-in
sounds, and external sound files (`mp3`, `ogg`, `flac` or `wav` placed inside
the `sounds` folder within the configuration directory). See
[notifications](/configuration/notifications/#sound) for a list of all built-in
sounds.

```toml
# Type: string
# Values: see above for built-in sounds, eg: "sing" or external sound.
# Default: not set

[[highlights.match]]
words = ["word1", "word2", "word3"]
sound = "sing"
```
