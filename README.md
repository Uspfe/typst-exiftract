# exiftract

Read the Exif metadata of an image — the camera, the exposure, the date, the
place it was taken — and get it back as Typst values you can typeset or
compute with.

The parsing happens in a WebAssembly plugin built from [kamadak-exif], so there
are no external tools to install and no shell-outs; it works the same in the
web app, in the CLI and in CI.

![A report built from an image's Exif metadata](examples/metadata.png)

```typst
#import "@preview/exiftract:0.1.0": read-exif

#let fields = read-exif(read("photo.jpg", encoding: none))
#let taken = fields.find(f => f.tag == "DateTimeOriginal").value

Taken on #taken.display("[day] [month repr:long] [year]").
```

Requires Typst 0.15 or later.

## `read-exif`

```typst
read-exif(
  data,
  return-raw: false,
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
  type: "rational",     // Exif type of the value, as stored
  count: 1,             // number of elements, before truncation
  value: 0.005,         // the value, as a Typst type
  unit: "s",            // its unit, or none
)
```

| Argument | Type | Meaning |
| --- | --- | --- |
| `data` | `bytes` | The raw image file, e.g. `read("photo.jpg", encoding: none)`. |
| `return-raw` | `bool` | Hand back what the file holds, uninterpreted. See [below](#return-raw). |
| `default` | any | Returned when the image has no readable Exif block. Left at `auto`, that case panics instead. Returned as given, never interpreted. |
| `max-values` | `int` | How many elements of a multi-valued field to keep. |
| `lenient` | `bool` | Keep what could be parsed from a damaged Exif block instead of treating the damage as an error. |

JPEG, TIFF (including TIFF-based raw formats), PNG, WebP and HEIF/HEIC/AVIF
containers are understood.

Typst resolves relative paths against the file they appear in, so pass the
bytes rather than a path — `read-exif("photo.jpg")` would look for the file
next to `lib.typ` and is rejected with a hint.

### Values

Exif stores everything as integers, rationals and byte strings. `read-exif`
turns those into the Typst type that carries the same meaning:

| Exif | becomes | example |
| --- | --- | --- |
| rational | `float` | `ExposureTime` `(1, 200)` → `0.005` |
| `DateTime`, `DateTimeOriginal`, `DateTimeDigitized` | `datetime` | `"2024:05:17 09:30:00"` |
| `GPSDateStamp` | `datetime` (date only) | `"2024:05:17"` |
| `GPSLatitude`, `GPSLongitude`, `GPSDestLatitude`, `GPSDestLongitude` | `angle` | `(48, 8, 41.23)` → `48.1448deg` |
| `GPSTrack`, `GPSImgDirection`, `GPSDestBearing`, `CameraElevationAngle` | `angle` | `270.5deg` |
| `UNDEFINED` | `bytes` | `ExifVersion` → `str(…)` is `"0232"` |
| integers, ASCII | unchanged | they are already native |

Coordinates are signed from their hemisphere: `GPSLatitudeRef` `"S"` and
`GPSLongitudeRef` `"W"` give a negative angle, so the value goes straight into
a map URL. `GPSAltitude` is negated when `GPSAltitudeRef` says below sea level.
Those neighbouring fields are looked up within the same image directory, so a
thumbnail's units never leak into the primary image's.

Fields with a single element — nearly all of them — are unwrapped to a bare
value; genuinely multi-valued ones stay arrays:

```typst
#by-tag.Make          // "Canon"
#by-tag.GPSTimeStamp  // (7.0, 30.0, 12.5)
```

`count` is the true number of elements either way, and stays correct when
`max-values` truncated the array. That limit exists because fields like
`MakerNote` and `UserComment` can run to tens of kilobytes of raw bytes; raise
it if you need them in full.

### `unit`

`unit` is a string, or `none` when the value is dimensionless or its Typst type
already implies the unit:

```typst
#let field = fields.find(f => f.tag == "FocalLength")
#field.value  // 35.0
#field.unit   // "mm"
```

The units come from the Exif specification. Some are fixed by tag — `"s"`,
`"mm"`, `"m"`, `"EV"`, `"pixels"`, `"hPa"` — and some are named by a
neighbouring field, which `read-exif` resolves for you:

| Tag | unit from | example |
| --- | --- | --- |
| `XResolution`, `YResolution` | `ResolutionUnit` | `"pixels per inch"` |
| `FocalPlaneXResolution`, `FocalPlaneYResolution` | `FocalPlaneResolutionUnit` | `"pixels per cm"` |
| `GPSSpeed` | `GPSSpeedRef` | `"km/h"` |
| `GPSDestDistance` | `GPSDestDistanceRef` | `"nautical miles"` |

### What is deliberately not converted

A conversion is only worth having if it holds for every file and every field of
that kind. Three cases fail that test:

**Lengths stay numbers.** Typst's `length` covers pt, mm, cm, in and em — but
not metres, and `SubjectDistance` is in metres. Converting `FocalLength` to
`35mm` while `SubjectDistance` had to stay a number would make the type of a
"length" field depend on which length it is. A Typst `length` is also a layout
dimension, not a physical quantity: `35mm` normalises to `99.21pt` and would
happily be used as a page width. So every length is a `float` with its unit in
`unit`.

**`GPSTimeStamp` stays three numbers.** It holds hours, minutes and seconds as
rationals, and the seconds are routinely fractional. Typst's `datetime` and
`duration` both take whole seconds only, so either conversion would work on
some files and round on others. It comes back as `(7.0, 30.0, 12.5)` with
`unit: "h, min, s"`.

**Enumerations stay numbers.** `Orientation` is `1`, not `"row 0 at top and
column 0 at left"`; `Flash` is a bit field. Those are presentation, and a
decoding table belongs above this package, not inside it.

### Values that do not convert cleanly

Reading never fails on a damaged file:

- `0/0`, which Exif uses for "unknown", becomes `float.nan` — test it with
  `float.is-nan(value)` rather than `==`.
- A date that does not parse, is blank, or does not exist — February 30th shows
  up in real files — stays the string it was. Check with
  `type(value) == datetime` before formatting.
- Unknown tags pass through untouched, with `unit: none`.

One thing to know about the datetimes: `GPSDateStamp` has no time of day, so
`display()` with an `[hour]` in the format string will fail on it.
`value.hour() == none` tells the two apart. Exif keeps sub-second digits and UTC
offsets in their own fields (`SubSecTimeOriginal`, `OffsetTimeOriginal`);
Typst's `datetime` cannot hold either, so they stay separate fields for you to
apply.

### `return-raw`

`return-raw: true` skips interpretation and hands back what the file holds.
Rationals stay `(numerator, denominator)` pairs, dates stay strings,
`UNDEFINED` stays a list of byte values, and no `unit` member is added. Every
other member is the same.

```typst
#read-exif(photo).find(f => f.tag == "ExposureTime").value
// 0.005

#read-exif(photo, return-raw: true).find(f => f.tag == "ExposureTime").value
// (1, 200)
```

Use it when you want the exact stored numerator and denominator, or when you
would rather interpret the fields your own way.

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

## Example

[`examples/metadata.typ`](examples/metadata.typ) is the document shown above:
the photo, a summary table and every field it found. It shows one way to render
the values, units and all.

## Building from source

The plugin's Rust source, the test suites and the build tooling live in the
[repository][repo]. `exif.wasm` is committed there, so the package works
without a Rust toolchain; to rebuild it:

```sh
rustup target add wasm32-unknown-unknown
make            # builds plugin/ and refreshes exif.wasm
make test       # Rust tests, then both Typst suites, then the staged package
make dist       # stages the package for submission
```

The test images are generated by `plugin/examples/mkfixtures.rs` rather than
checked in as photographs, so the metadata under test is explicit and the
repository stays small.

## License

The Typst sources are MIT — see [LICENSE](LICENSE).

`exif.wasm` additionally contains the Rust crates it was compiled from, chiefly
[kamadak-exif], which is BSD-2-Clause. Their notices are reproduced in
[NOTICE.md](NOTICE.md), and the manifest declares the combination as
`MIT AND BSD-2-Clause`.

[kamadak-exif]: https://github.com/kamadak/exif-rs
[repo]: https://github.com/Uspfe/typst-exif
