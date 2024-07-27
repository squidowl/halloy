# Text Formatting

Text can be formatted in Halloy by using the `/format` (or `/f`) command.

## Attributes

Below is a table with the supported text attributes.

| Action                | Markdown                | Token                     |
| --------------------- | ----------------------- | ------------------------- |
| _Italics_             | `_italic text_`         | `$iitalic text$i`         |
| **Bold**              | `__bold text__`         | `$bbold text$b`           |
| **_Italic and Bold_** | `___italic and bold___` | `$b$iitalic and bold$i$b` |
| Code                  | `` `code` ``            | `$mcode$m`                |
| Spoiler               | `\|\|spoiler\|\|`       | -                         |

Example

```json
/format __this is bold__ $iand this is italic$i
```

Will render the following: 
> __this is bold__ _and this is italic_

## Color

| Action                        | Token   |
| ----------------------------- | ------- |
| Text color (fg)               | `$c0`   |
| Text and background (fg & bg) | `$c0,1` |
| End color                     | `$c`    |

The number next to the `$c` token indicates the color. For a comprehensive list of all numbers, see the following [ircdocs.horse documentation](https://modern.ircdocs.horse/formatting#colors-16-98). Below, the first 00 to 15 colors are defined and have been assigned aliases for convenience.

Colors

<span style="display:inline-block;width:12px;height:12px;background-color:#ffffff;"></span> - 00 - white  
<span style="display:inline-block;width:12px;height:12px;background-color:#000000;"></span> - 01 - black  
<span style="display:inline-block;width:12px;height:12px;background-color:#00007f;"></span> - 02 - blue  
<span style="display:inline-block;width:12px;height:12px;background-color:#009300;"></span> - 03 - green  
<span style="display:inline-block;width:12px;height:12px;background-color:#ff0000;"></span> - 04 - red  
<span style="display:inline-block;width:12px;height:12px;background-color:#7f0000;"></span> - 05 - brown  
<span style="display:inline-block;width:12px;height:12px;background-color:#9c009c;"></span> - 06 - magenta  
<span style="display:inline-block;width:12px;height:12px;background-color:#fc7f00;"></span> - 07 - orange  
<span style="display:inline-block;width:12px;height:12px;background-color:#ffff00;"></span> - 08 - yellow  
<span style="display:inline-block;width:12px;height:12px;background-color:#00fc00;"></span> - 09 - lightgreen  
<span style="display:inline-block;width:12px;height:12px;background-color:#009393;"></span> - 10 - cyan  
<span style="display:inline-block;width:12px;height:12px;background-color:#00ffff;"></span> - 11 - lightcyan  
<span style="display:inline-block;width:12px;height:12px;background-color:#0000fc;"></span> - 12 - lightblue  
<span style="display:inline-block;width:12px;height:12px;background-color:#ff00ff;"></span> - 13 - pink  
<span style="display:inline-block;width:12px;height:12px;background-color:#7f7f7f;"></span> - 14 - grey  
<span style="display:inline-block;width:12px;height:12px;background-color:#d2d2d2;"></span> - 15 - lightgrey  

Example

```
/format $cred,lightgreenfoobar$c
/format $c04,09foobar$c
```

Will both render the following: 

<span style="display: inline-block; background-color: #00fc00; color: #ff0000;">
  foobar
</span>


## Configuration

By default, Halloy will only format text when using the `/format` command. This, however, can be changed with the `auto_format` configuration option:

```toml
[buffer.text_input]
auto_format: "disabled" | "markdown" | "all"
```
