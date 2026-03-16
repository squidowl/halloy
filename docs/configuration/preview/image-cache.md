# Image Cache

Settings to control how the image cache is managed.  The cache is stored in:

* Windows: `%AppData%\Roaming\Local\halloy\previews\images\`
* Mac: `~/Library/Caches/halloy/previews/images/` or `$HOME/.cache/halloy/previews/images/`
* Linux: `$XDG_CACHE_HOME/halloy/previews/images/`, `$HOME/.cache/halloy/previews/images/`, or `$HOME/.var/app/org.squidowl.halloy/cache/halloy/previews/images/` (Flatpak)

## max_size

Maximum size in MB for cached preview images, or `"unlimited"` for an uncapped image cache (not recommended).

```toml
# Type: integer
# Values: any non-negative integer or "unlimited"
# Default: 500

[preview.request.image_cache]
max_size = 500
```

## trim_interval

Run image cache trimming every N successful image saves. Set to `"first-save-only"` to disable periodic trimming, and only trim on the first save to the image cache per app session.

```toml
# Type: integer
# Values: any non-negative integer or "first-save-only"
# Default: 32

[preview.request.image_cache]
trim_interval = 32
```
