# Autocomplete

Customize autocomplete.

- [Autocomplete](#autocomplete)
  - [Configuration](#configuration)
    - [order\_by](#order_by)
    - [sort\_direction](#sort_direction)
    - [completion\_suffixes](#completion_suffixes)

## Configuration

### order_by

Ordering that autocomplete uses to select from matching users.

- `"recent"`: Autocomplete users by their last message in the channel;  the user with most recent message autocompletes first, then increasingly older messages.  Users with no seen messages are matched last, in the order specified by `sort_direction`.
- `"alpha"`: Autocomplete users based on alphabetical ordering of potential matches.  Ordering is ascending/descending based on `sort_direction`.

```toml
# Type: string
# Values: "alpha", "recent"
# Default: "recent"

[buffer.text_input.autocomplete]
order_by = "recent"
```

### sort_direction

Sort direction when autocompleting alphabetically.

- `"asc"`: ascending alphabetical (a→z)
- `"desc"`: descending alphabetical (z→a)

```toml
# Type: string
# Values: "asc", "desc"
# Default: "asc"

[buffer.text_input.autocomplete]
sort_direction = "asc"
```

### completion_suffixes

Sets what suffix is added after autocompleting. The first option is for when a nickname is autocompleted at the beginning of a sentence. The second is for when it's autocompleted in the middle of a sentence.

```toml
# Type: array of 2 strings
# Values: array of 2 strings
# Default: [": ", " "]

[buffer.text_input.autocomplete]
completion_suffixes = [": ", " "]
```
