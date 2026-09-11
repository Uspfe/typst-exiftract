# typst-exif

Read Exif metadata from images in [Typst](https://typst.app).

The parsing happens in a WebAssembly plugin built from [kamadak-exif], so no
external tools, no shell-outs, and it works the same in the web app, in the
CLI and in CI.

```typst
#import "@preview/exif:0.1.0": read-exif

#let fields = read-exif(read("photo.jpg", encoding: none))

#for field in fields [
  / #field.tag: #repr(field.value)
]
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
  lenient: true,
) -> array
```

The package exposes this one function. It returns the Exif fields in the order
the file stores them, one dictionary each:

```typst
(
  tag: "ExposureTime",  // tag name, or e.g. "exif-0x9999" if unknown
  ifd: "exif",          // primary, thumbnail, exif, gps or interop
  number: 33434,        // numeric tag id
  type: "rational",     // Exif type of the value
  count: 1,             // number of elements, before truncation
  value: (1, 200),      // the value as the file stores it
)
```

| Argument | Type | Meaning |
| --- | --- | --- |
| `data` | `bytes` | The raw image file, e.g. `read("photo.jpg", encoding: none)`. |
| `default` | any | Returned when the image has no readable Exif block. Left at `auto`, that case panics instead. |
| `max-values` | `int` | How many elements of a multi-valued field to keep. |
| `lenient` | `bool` | Keep what could be parsed from a damaged Exif block instead of treating the damage as an error. |

JPEG, TIFF (including TIFF-based raw formats), PNG, WebP and HEIF/HEIC/AVIF
containers are understood.

Typst resolves relative paths against the file they appear in, so pass the
bytes rather than a path — `read-exif("photo.jpg")` would look for the file
next to `lib.typ` and is rejected with a hint.

### Values

Values come back as the file holds them, with no interpretation: integers stay
integers, ASCII stays a string, a rational stays a `(numerator, denominator)`
pair, and `UNDEFINED` stays an array of byte values. `type` says which of the
twelve Exif types it was.

```typst
"Canon"       // ascii
1             // short   — Orientation
(1, 200)      // rational — ExposureTime, exactly as stored
```

Nothing is rounded on the way out, so `ExposureTime` is `(1, 200)`, not
`0.005`. Divide when you want the number:

```typst
#let (num, denom) = field.value
#(num / denom)
```

Fields with a single element — nearly all of them — are unwrapped to a bare
value; genuinely multi-valued ones stay arrays:

```typst
#by-tag.Make          // "Canon"
#by-tag.GPSLatitude   // ((48, 1), (8, 1), (4123, 100))
```

`count` is the true number of elements either way, and stays correct when
`max-values` truncated the array. That limit exists because fields like
`MakerNote` and `UserComment` can run to tens of kilobytes of raw bytes; raise
it if you need them in full.

### Looking a tag up

The result is a plain array, so use the array methods:

```typst
#let fields = read-exif(read("photo.jpg", encoding: none))

#fields.find(f => f.tag == "Model").value
#fields.filter(f => f.ifd == "gps")
```

For repeated lookups, build a dictionary once:

```typst
#let by-tag = fields.map(f => (f.tag, f.value)).to-dict()
#by-tag.at("Model", default: "unknown camera")
```

Tags the parser does not know are keyed by context and number instead, like
`tiff-0x9999`. The primary image and its embedded thumbnail can both carry the
same tag; they stay separate entries, told apart by `ifd`.

### Images without Exif

By default a missing or unreadable Exif block is an error:

```typst
#read-exif(read("screenshot.png", encoding: none))
// error: could not read Exif metadata: No Exif data found in PNG
```

Pass `default` to handle it in the document instead:

```typst
#let fields = read-exif(read(path, encoding: none), default: ())
#if fields == () [No metadata.]
```

### Dates

`DateTimeOriginal` is the raw Exif string, `"2024:05:17 09:30:00"`. To get a
`datetime`:

```typst
#let (date, time) = by-tag.DateTimeOriginal.split(" ")
#let (y, mo, d) = date.split(":").map(int)
#let (h, mi, s) = time.split(":").map(int)
#datetime(year: y, month: mo, day: d, hour: h, minute: mi, second: s)
```

### Example

[`examples/metadata.typ`](examples/metadata.typ) builds a small report — the
photo, a summary table and every field it found — including one way to render
rationals as fractions.

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
