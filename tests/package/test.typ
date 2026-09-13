// Compiles against the *installed* package through an @preview import, with
// no access to the working tree — the same path a user takes. This is what
// catches a file missing from the staged bundle.
//
//   make test-package

#import "@preview/exiftract:0.1.0": read-exif

#let fields = read-exif(read("photo.jpg", encoding: none))

// The README's opening snippet, with the outputs it claims.
#let by-tag = fields.map(f => (f.tag, f.value)).to-dict()
#assert.eq(by-tag.Model, "Exif Fixture Camera")
#assert.eq(by-tag.ExposureTime, 0.005)
#assert.eq(by-tag.DateTimeOriginal.display(), "2024-05-17 09:30:00")
#assert.eq(calc.round(by-tag.GPSLatitude.deg(), digits: 4), 48.1448)

// The claim that a dictionary keeps the last of two same-named fields.
#assert.eq((("a", 1), ("a", 2)).to-dict(), (a: 2))

// A representative value of each other interpreted type.
#assert.eq(by-tag.Make, "Typst")
#assert.eq(type(by-tag.GPSLatitude), angle)
#assert.eq(str(by-tag.ExifVersion), "0232")
#assert.eq(fields.find(f => f.tag == "FocalLength").unit, "mm")

// And the raw form.
#let raw = read-exif(read("photo.jpg", encoding: none), return-raw: true)
#assert.eq(raw.find(f => f.tag == "ExposureTime").value, (1, 200))

The package works through the preview namespace.
