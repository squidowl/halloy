# `[buffer.text_input_autocomplete]`

Customize autocomplete.

**Example**

```toml
[buffer.text_input.autocomplete]
sort_direction = "asc"
completion_suffixes = [": ", ""]
```

## `sort_direction`

Sort direction when autocompleting.

- **type**: string
- **values**: `"asc"`, `"desc"`
- **default**: `"asc"`

## `completion_suffixes`

Sets what suffix is added after autocompleting. The first option is for when a nickname is autocompleted at the beginning of a sentence. The second is for when it's autocompleted in the middle of a sentence.

- **type**: array of 2 strings
- **values**: array of 2 strings
- **default**: `"[": ", " "]"`
