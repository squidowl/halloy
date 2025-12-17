# Preview

URL preview settings for Halloy.

- [Preview](#preview)
  - [Configuration](#configuration)
    - [enabled](#enabled)
  - [Request](#request)
  - [Image](#image)
  - [Card](#card)

## Configuration

### enabled

Enable or disable previews globally with a boolean, or selectively enable them for URLs matching specific regex patterns.

```toml
# Type: boolean or array of strings
# Values: true, false, or array of regex patterns
# Default: true

[preview]
enabled = true
```

Only show previews for matching URLs:

> 💡 Use toml multi-line literal strings `'''\bfoo'd\b'''` when writing a regex. This allows you to write the regex without escaping. You can also use a literal string `'\bfoo\b'`, but then you can't use `'` inside the string.
>
> Without literal strings, you'd have to write the above as `"\\bfoo'd\\b"`

```toml
[preview]
enabled = [
    '''https?://(www\.)?imgur\.com/.*''', 
    '''https?://(www\.)?dr\.dk/.*'''
]
```

## [Request](request.md)

Request settings for previews.

## [Image](image.md)

Specific image preview settings.

## [Card](card.md)

Specific card preview settings.
