// Typst-side test suite. Run with:
//   typst compile --root . --format pdf tests/test.typ /dev/null
// Every assertion that fails aborts the compilation with a message.

#import "/lib.typ": read-exif

#let jpeg = read("/tests/assets/sample.jpg", encoding: none)
#let png = read("/tests/assets/sample.png", encoding: none)
#let plain = read("/tests/assets/no-exif.jpg", encoding: none)

#let fields = read-exif(jpeg)
#let field-named(name) = fields.find(f => f.tag == name)
#let by-tag = fields.map(f => (f.tag, f.value)).to-dict()

// --- the shape of the result -----------------------------------------------

#assert.eq(type(fields), array)
#assert.eq(fields.len(), 29)
#assert.eq(
  fields.first().keys().sorted(),
  ("count", "ifd", "number", "tag", "type", "value"),
)

// Fields come in file order: the primary IFD, then the Exif and GPS sub-IFDs.
#assert.eq(
  fields.map(f => f.ifd).dedup(),
  ("primary", "exif", "gps"),
)

// --- values are what the file holds ----------------------------------------

#assert.eq(by-tag.Make, "Typst")
#assert.eq(by-tag.Model, "Exif Fixture Camera")
#assert.eq(by-tag.DateTimeOriginal, "2024:05:17 09:30:00")
#assert.eq(by-tag.Orientation, 1)
#assert.eq(by-tag.PixelXDimension, 16)
#assert.eq(type(by-tag.Make), str)
#assert.eq(type(by-tag.Orientation), int)

// Rationals keep numerator and denominator, so nothing is rounded away.
#assert.eq(by-tag.ExposureTime, (1, 200))
#assert.eq(by-tag.FNumber, (28, 10))
#assert.eq(by-tag.XResolution, (72, 1))

// --- multi-valued fields ---------------------------------------------------

#assert.eq(by-tag.GPSLatitude, ((48, 1), (8, 1), (4123, 100)))
#assert.eq(by-tag.GPSLatitudeRef, "N")
#assert.eq(field-named("GPSLatitude").count, 3)

// --- per-field records -----------------------------------------------------

#let make = field-named("Make")
#assert.eq(make.ifd, "primary")
#assert.eq(make.type, "ascii")
#assert.eq(make.count, 1)
#assert.eq(make.number, 271)
#assert.eq(make.value, "Typst")

#assert.eq(field-named("GPSLatitude").ifd, "gps")
#assert.eq(field-named("GPSLatitude").type, "rational")
#assert.eq(field-named("DateTimeOriginal").ifd, "exif")
#assert.eq(field-named("Orientation").type, "short")

// --- other containers ------------------------------------------------------

#assert.eq(read-exif(png), fields)

// --- images without Exif ---------------------------------------------------

#assert.eq(read-exif(plain, default: none), none)
#assert.eq(read-exif(plain, default: ()), ())
#assert.eq(read-exif(bytes("neither jpeg nor tiff"), default: none), none)

// --- limits ----------------------------------------------------------------

#let capped = read-exif(jpeg, max-values: 2).find(f => f.tag == "GPSLatitude")
#assert.eq(capped.value, ((48, 1), (8, 1)))
#assert.eq(capped.count, 3)

// `max-values: 0` keeps scalars — they are not vectors from the reader's
// point of view — but empties every genuinely multi-valued field.
#let none-kept = read-exif(jpeg, max-values: 0).map(f => (f.tag, f.value)).to-dict()
#assert.eq(none-kept.Make, "Typst")
#assert.eq(none-kept.GPSLatitude, ())

All Typst tests passed.
