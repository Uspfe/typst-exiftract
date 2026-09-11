# typst-exif

Read Exif metadata from images in [Typst](https://typst.app).

The parsing happens in a WebAssembly plugin built from [kamadak-exif], so no
external tools, no shell-outs, and it works the same in the web app, in the
CLI and in CI.

```typst
#import "@preview/exif:0.1.0": read-exif

#let data = read-exif(read("photo.jpg", encoding: none))

Shot on #data.display.Model at #data.display.ExposureTime, f/#data.tags.FNumber.
```

Requires Typst 0.15 or later.

## Installing

Until the package is on Typst Universe, vendor it into your project and import
it by path:

```typst
#import "typst-exif/lib.typ": read-exif
```

`lib.typ` and `exif.wasm` must sit next to each other; nothing else is needed
at runtime.

## `read-exif`

```typst
read-exif(
  data,
  default: auto,
  max-values: 64,
  max-display: 256,
  lenient: true,
) -> dictionary
```

The package exposes this one function.

| Argument | Type | Meaning |
| --- | --- | --- |
| `data` | `bytes` | The raw image file, e.g. `read("photo.jpg", encoding: none)`. |
| `default` | any | Returned when the image has no readable Exif block. Left at `auto`, that case panics instead. |
| `max-values` | `int` | How many elements of a multi-valued field to keep. |
| `max-display` | `int` | How many characters of a `display` string to keep. |
| `lenient` | `bool` | Keep what could be parsed from a damaged Exif block instead of failing. |

JPEG, TIFF (including TIFF-based raw formats), PNG, WebP and HEIF/HEIC/AVIF
containers are understood.

Typst resolves relative paths against the file they appear in, so pass the
bytes rather than a path — `read-exif("photo.jpg")` would look for the file
next to `lib.typ` and is rejected with a hint.

### What you get back

```typst
(
  format: "jpeg",            // container the data was found in
  little-endian: true,       // byte order of the Exif block
  count: 19,                 // number of fields
  tags: (Make: "Typst", ExposureTime: 0.005, ...),
  display: (Make: "Typst", ExposureTime: "1/200 s", ...),
  ifds: (primary: (...), exif: (...), gps: (...), thumbnail: (...)),
  fields: ((tag: "Make", ifd: "primary", ...), ...),
  warnings: (),              // non-fatal problems, when `lenient` is on
)
```

`tags` is the flat, machine-readable view: strings stay strings, integers stay
integers, and rationals such as `ExposureTime` become floats. `display` holds
the same fields rendered for a reader — `"1/200 s"`, `"f/2.8"`, `"72 pixels
per inch"`, `"row 0 at top and column 0 at left"` — with units and enumerated
values spelled out. Use `tags` to compute, `display` to typeset.

Both are keyed by the Exif tag name (`Make`, `DateTimeOriginal`,
`GPSLatitude`, …). Tags the parser does not know are keyed by context and
number instead, like `tiff-0x9999`. When the primary image and the embedded
thumbnail both carry a field, the primary one wins; `ifds` keeps them apart.

`fields` is the complete, ordered list, one dictionary per field:

| Key | Type | Meaning |
| --- | --- | --- |
| `tag` | `str` | Tag name, the key used in `tags`. |
| `ifd` | `str` | `"primary"`, `"thumbnail"`, `"exif"`, `"gps"` or `"interop"`. |
| `number` | `int` | Numeric tag id. |
| `type` | `str` | Exif type: `"ascii"`, `"short"`, `"rational"`, `"undefined"`, … |
| `count` | `int` | Number of elements, before any truncation. |
| `value` | any | Same value as in `tags`. |
| `display` | `str` | Same string as in `display`. |
| `description` | `str` | Human-readable name, absent for unknown tags. |
| `truncated` | `bool` | Only present, and `true`, when `max-values` dropped elements. |

Single-element fields — nearly all of them — are unwrapped to a bare value;
genuinely multi-valued ones stay arrays:

```typst
#data.tags.Make          // "Typst"
#data.tags.GPSLatitude   // (48.0, 8.0, 41.23)
```

`count` tells you the real length either way. `max-values` exists because
fields like `MakerNote` and `UserComment` can run to tens of kilobytes of raw
bytes; raise it if you need them in full.

### Images without Exif

By default a missing or unreadable Exif block is an error:

```typst
#read-exif(read("screenshot.png", encoding: none))
// error: could not read Exif metadata: No Exif data found in PNG
```

Pass `default` to handle it in the document instead:

```typst
#let data = read-exif(read(path, encoding: none), default: none)
#if data == none [No metadata.] else [Shot on #data.display.Model.]
```

### Dates

`tags.DateTimeOriginal` is the raw Exif string, `"2024:05:17 09:30:00"`;
`display.DateTimeOriginal` normalises it to `"2024-05-17 09:30:00"`. To get a
`datetime`:

```typst
#let (date, time) = data.tags.DateTimeOriginal.split(" ")
#let (y, mo, d) = date.split(":").map(int)
#let (h, mi, s) = time.split(":").map(int)
#datetime(year: y, month: mo, day: d, hour: h, minute: mi, second: s)
```

### Example

[`examples/metadata.typ`](examples/metadata.typ) builds a small report — the
photo, a summary table and every field it found.

## Building from source

`exif.wasm` is committed so that the package works out of the box. To rebuild
it:

```sh
rustup target add wasm32-unknown-unknown
make            # builds plugin/ and refreshes exif.wasm
make test       # Rust tests, then the Typst test suite
```

`make fixtures` regenerates the images in `tests/assets/`; they are written by
`plugin/examples/mkfixtures.rs` rather than checked in as photographs, so the
metadata under test is explicit and the repository stays small.

## Layout

| Path | |
| --- | --- |
| `lib.typ` | the Typst API |
| `exif.wasm` | the compiled plugin |
| `plugin/` | its Rust source and tests |
| `tests/` | Typst test suite and generated fixtures |
| `examples/` | example document |

## License

MIT, see [LICENSE](LICENSE). The plugin links [kamadak-exif], which is
BSD-2-Clause.

[kamadak-exif]: https://github.com/kamadak/exif-rs
