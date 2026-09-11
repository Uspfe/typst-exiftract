// Compiles against the *installed* package through an @preview import, with
// no access to the working tree — the same path a user takes. This is what
// catches a file missing from the staged bundle.
//
//   make test-package

#import "@preview/exiftract:0.1.0": read-exif

#let fields = read-exif(read("photo.jpg", encoding: none))

// The README's opening example, verbatim.
#let taken = fields.find(f => f.tag == "DateTimeOriginal").value
#assert.eq(taken.display("[day] [month repr:long] [year]"), "17 May 2024")

// A representative value of each interpreted type.
#let by-tag = fields.map(f => (f.tag, f.value)).to-dict()
#assert.eq(by-tag.Make, "Typst")
#assert.eq(by-tag.ExposureTime, 0.005)
#assert.eq(type(by-tag.GPSLatitude), angle)
#assert.eq(str(by-tag.ExifVersion), "0232")
#assert.eq(fields.find(f => f.tag == "FocalLength").unit, "mm")

// And the raw form.
#let raw = read-exif(read("photo.jpg", encoding: none), return-raw: true)
#assert.eq(raw.find(f => f.tag == "ExposureTime").value, (1, 200))

The package works through the preview namespace.
