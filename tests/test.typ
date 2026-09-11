// Typst-side test suite. Run with:
//   typst compile --root . --format pdf tests/test.typ /dev/null
// Every assertion that fails aborts the compilation with a message.

#import "/lib.typ": read-exif

#let jpeg = read("/tests/assets/sample.jpg", encoding: none)
#let png = read("/tests/assets/sample.png", encoding: none)
#let plain = read("/tests/assets/no-exif.jpg", encoding: none)

// --- the shape of the returned dictionary ---------------------------------

#let data = read-exif(jpeg)

#assert.eq(type(data), dictionary)
#assert.eq(
  data.keys().sorted(),
  ("count", "display", "fields", "format", "ifds", "little-endian", "tags", "warnings"),
)
#assert.eq(data.format, "jpeg")
#assert.eq(data.at("little-endian"), true)
#assert.eq(data.count, data.fields.len())
#assert.eq(data.warnings, ())

// --- flat tag lookup -------------------------------------------------------

#assert.eq(data.tags.Make, "Typst")
#assert.eq(data.tags.Model, "Exif Fixture Camera")
#assert.eq(data.tags.DateTimeOriginal, "2024:05:17 09:30:00")
#assert.eq(data.tags.Orientation, 1)
#assert.eq(data.tags.PixelXDimension, 16)
#assert.eq(type(data.tags.Make), str)
#assert.eq(type(data.tags.Orientation), int)

// Rationals arrive as floats; the fraction survives in `display`.
#assert.eq(data.tags.ExposureTime, 1 / 200)
#assert.eq(data.tags.FNumber, 2.8)
#assert.eq(data.display.ExposureTime, "1/200 s")
#assert.eq(data.display.FNumber, "f/2.8")

// Enumerated values are spelled out in `display`.
#assert.eq(data.display.Orientation, "row 0 at top and column 0 at left")
#assert.eq(data.display.ResolutionUnit, "inch")

// --- multi-valued fields ---------------------------------------------------

#assert.eq(data.tags.GPSLatitude, (48.0, 8.0, 41.23))
#assert.eq(data.tags.GPSLatitudeRef, "N")
#assert.eq(data.display.GPSLatitude, "48 deg 8 min 41.23 sec N")

// --- per-field records -----------------------------------------------------

#let make = data.fields.find(f => f.tag == "Make")
#assert.eq(
  make.keys().sorted(),
  ("count", "description", "display", "ifd", "number", "tag", "type", "value"),
)
#assert.eq(make.ifd, "primary")
#assert.eq(make.type, "ascii")
#assert.eq(make.count, 1)
#assert.eq(make.number, 271)
#assert.eq(make.value, "Typst")

#let latitude = data.fields.find(f => f.tag == "GPSLatitude")
#assert.eq(latitude.ifd, "gps")
#assert.eq(latitude.count, 3)

// --- grouping by IFD -------------------------------------------------------

#assert.eq(data.ifds.keys().sorted(), ("exif", "gps", "primary"))
#assert.eq(data.ifds.primary.Make, "Typst")
#assert.eq(data.ifds.exif.DateTimeOriginal, "2024:05:17 09:30:00")
#assert.eq(data.ifds.gps.GPSLongitudeRef, "E")
#assert("Make" not in data.ifds.exif)

// --- other containers ------------------------------------------------------

#let from-png = read-exif(png)
#assert.eq(from-png.format, "png")
#assert.eq(from-png.tags.Make, "Typst")
#assert.eq(from-png.tags, data.tags)

// --- images without Exif ---------------------------------------------------

#assert.eq(read-exif(plain, default: none), none)
#assert.eq(read-exif(plain, default: (tags: (:))), (tags: (:)))
#assert.eq(read-exif(bytes("neither jpeg nor tiff"), default: none), none)

// --- limits ----------------------------------------------------------------

#let capped = read-exif(jpeg, max-values: 2, max-display: 4)
#let latitude = capped.fields.find(f => f.tag == "GPSLatitude")
#assert.eq(latitude.value, (48.0, 8.0))
#assert.eq(latitude.count, 3)
#assert.eq(latitude.truncated, true)
#assert.eq(capped.display.Model, "Exif…")

// `max-values: 0` keeps scalars (they are not vectors from the reader's point
// of view) but empties every genuinely multi-valued field.
#let none-kept = read-exif(jpeg, max-values: 0)
#assert.eq(none-kept.tags.Make, "Typst")
#assert.eq(none-kept.tags.GPSLatitude, ())

All Typst tests passed.
