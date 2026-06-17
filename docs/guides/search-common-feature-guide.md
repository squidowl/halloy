# Halloy Search And Common Channel Feature Guide

Updated: 2026-06-16 EDT

This document describes the local `/search`, `/last`, and `/common` features
added in the Halloy feature branch. It is written as both user documentation
and a portable design reference for possible future implementations in other
IRC clients such as Konversation or Uplink.

## Security Model

These commands are local inspection tools. They must not send IRC traffic by
default.

- `/search` and `/last` inspect already-loaded local Halloy history only.
- `/common` inspects already-known in-memory channel membership only.
- `/common` does not issue `WHOIS`, `WHO`, `NAMES`, or other network refreshes.
- Search result output is local status output or a local result pane.
- Saved searches, filesystem scans, persistent result storage, and active
  identity refresh are separate future features.

Future network-enrichment modes, such as `/common --whois`, should be explicit,
rate-limited, and documented as active network behavior.

## `/search`

`/search` searches the current buffer's loaded visible history.

```text
/search text=timeout
/search itext=timeout
/search regex="tim(e|ed) out"
/search origin=alice text=deploy
```

Bare text is treated as a message-body search:

```text
/search timeout
```

Selector text inside quotes is literal body text:

```text
/search origin="alice"
/search "origin=alice"
```

The first command filters by origin. The second command searches message text
for the literal string `origin=alice`.

## `/last`

`/last` is retained as a convenience form of search for the current buffer.
In the current implementation it searches the current buffer's loaded visible
history. It is not yet anchored to a HexChat-style read marker.

Planned future behavior: once read markers exist, `/last` should search from
the last-viewed/read-marker boundary to the present by default.

## Search Selectors

| Selector | Aliases | Value |
| --- | --- | --- |
| `text` | `itext`, `regex`, `iregex`, `regexp`, `re`, `rx`, `exp` | string |
| `origin` | `from`, `sender`, `nick`, `name` | nickname |
| `target` | `to` | nickname or channel |
| `type` | `kind` | message type |
| `span` | `since` | duration |
| `reaction` | `react` | reaction name |

Examples:

```text
/search nick=alice
/search react=love
/search reaction=thumbsup
/search type=action text=deploy
```

## Boolean Expressions

Search expressions support explicit boolean operators and parentheses.

```text
/search text=deploy AND origin=alice
/search text=deploy OR text=rollback
/search text=deploy AND NOT origin=bob
/search (text=deploy OR text=rollback) AND origin=alice
```

Adjacent predicates default to `AND`:

```text
/search origin=alice text=deploy
```

## String Modifiers

Any string-valued selector can use compact modifiers before a quoted value:

| Modifier | Meaning |
| --- | --- |
| `i` | case-insensitive |
| `n` | negated |
| `a` | comma-list values use AND |
| `o` | comma-list values use OR |
| `x` | regular expression |

Examples:

```text
/search text=i"timeout"
/search text=x"tim(e|ed) out"
/search text=ix"tim(e|ed) out"
/search text=n"noise"
/search reaction=o"love,thumbsup"
```

The modifier form is intended to be portable across selectors:

```text
/search origin=i"alice"
/search target=i"#rust"
/search reaction=i"love"
```

## Quoting And Escapes

Inside quoted values:

- `\"` means a literal double quote.
- `\\` means a literal backslash.
- Unknown escape sequences preserve the backslash and following character.

Examples:

```text
/search text="he said \"hello\""
/search text="path C:\\tmp"
```

## Span

`span=` accepts positive duration values with a required unit:

| Example | Meaning |
| --- | --- |
| `span=3d` | last 3 days |
| `span=2h` | last 2 hours |
| `span=5m` | last 5 minutes |

Examples:

```text
/search span=3d text=release
/last span=2h itext=error
```

## Search Output Options

| Option | Meaning |
| --- | --- |
| `--textonly` | strips IRC formatting controls and ANSI color/control sequences |
| `--notimestamp` | omits timestamps from output |
| `--other` | excludes messages uttered by the local user |
| `context=<lines>` | includes loaded lines before and after each match |
| `view=inline` | outputs local status lines in the current buffer |
| `view=pane` | opens a transient local result pane |
| `view=tab` | parsed but not implemented yet |

Examples:

```text
/search --textonly itext=error
/search --notimestamp text=deploy
/search --other text=deploy
/search context=2 text=panic
/search view=pane itext=timeout
```

Timestamps are displayed to whole seconds:

```text
[2026-06-16 16:01:17 UTC] <alice> example
```

With `--notimestamp`:

```text
<alice> example
```

## Highlighting

`view=pane` highlights positive message-body text predicate matches.

Examples:

```text
/search view=pane text=timeout
/search view=pane itext=timeout
/search view=pane regex="tim(e|ed) out"
```

Current scope:

- Highlights body text matches.
- Highlights repeated matches.
- Highlights multiple positive text predicates.
- Does not yet highlight origin, target, type, or reaction fields.
- Inline output is plain local status text.

## `/common`

`/common` lists users in the current channel who share other known channels with
you. It uses Halloy's in-memory membership state only.

```text
/common
/common scope=network
/common scope=global
```

`scope=global` is the default.

## `/common` Scope

| Scope | Meaning | Display |
| --- | --- | --- |
| `scope=network` | current network only | `nick: #channel` |
| `scope=global` | all connected networks | `network/nick: #channel` |

Examples:

```text
/common scope=network
alice: #rust, #linux
```

```text
/common
libera/alice: #rust
oftc/alice: #debian
```

Rules:

- `/common` works only from a channel buffer.
- The local user's nick is excluded.
- The current channel is excluded from the displayed overlap.
- Users with no other shared known channels are omitted.
- Shared channel names are sorted alphabetically.
- Result rows are sorted by display nick.
- No network refresh is performed.

## Future `/common` Identity Enrichment

The current `/common` implementation matches by nick across known membership
state. A future enrichment slice can improve identity matching by using
attributes Halloy already has in memory:

- nick
- username
- hostname
- account name

An active `WHOIS` pass can provide stronger correlation on some networks, but
it should be opt-in, explicit, and rate-limited:

```text
/common --whois
```

This future mode should clearly indicate that it sends IRC traffic.

## Halloy Implementation Notes

The current Halloy implementation keeps most feature logic in added modules:

- `data/src/command/search/`
- `data/src/command/common.rs`
- `src/buffer/common.rs`
- `src/buffer/search_results.rs`

Existing Halloy files are used mainly for parser registration, command
dispatch, buffer event plumbing, and pane rendering.

The current implementation intentionally avoids:

- persistent saved search storage;
- active log/file scans;
- active WHOIS fan-out;
- first-class `view=tab` search-result buffers;
- read-marker based `/last`.

## Upstream Contribution Notes

Halloy's current contribution guidance says significant new features should be
discussed before submission and that patches are submitted through GitHub pull
requests. If this work adds user-facing settings later, the related website
Markdown documentation should be updated along with the code.

Halloy's contribution page also currently says submitted AI-generated content
is not allowed. This document is therefore a private design/support artifact for
review, discussion, and possible porting. Any upstream submission should be
human-owned, reviewed, and rewritten as needed to comply with the project's
rules at the time of submission.

Reference: <https://halloy.chat/contributing#patches-pull-requests>

## Porting Notes

For Konversation, Uplink, or another IRC client, keep the same major seams:

- parse command syntax into typed selectors/options;
- evaluate only local history or explicitly requested network data;
- keep formatting/highlighting separate from evaluation;
- make network-enriching modes opt-in;
- keep default behavior local and side-effect free;
- expose `scope=network|global` for common-channel matching.

The portable command grammar is more important than the Halloy-specific UI
wiring. A different client can render results in a tab, pane, dialog, or inline
buffer while retaining the same parser and security model.
