// A small report built from an image's Exif metadata.
//
//   typst compile --root .. examples/metadata.typ

#import "/lib.typ": read-exif

#set page(width: 15cm, height: auto, margin: 1.5cm)
#set text(size: 10pt)

#let photo = read("/tests/assets/sample.jpg", encoding: none)
#let fields = read-exif(photo)

// Formatting is up to the document. Here: rationals as fractions when their
// denominator is not one, everything else as-is.
#let show-value(field) = {
  let one(v) = if type(v) == array and field.type.ends-with("rational") {
    let (num, denom) = v
    if denom == 1 { str(num) } else if num == 1 { $1 slash denom$ } else {
      str(num / denom)
    }
  } else {
    str(v)
  }

  if field.count == 1 { one(field.value) } else {
    field.value.map(one).join(", ")
  }
}

= #fields.find(f => f.tag == "ImageDescription").value

#grid(
  columns: (auto, 1fr),
  column-gutter: 1em,
  image(bytes(photo), width: 3cm),
  table(
    columns: 2,
    stroke: none,
    align: (right, left),
    ..fields
      .filter(f => f.tag in ("Make", "Model", "DateTimeOriginal", "ExposureTime", "FNumber"))
      .map(f => (strong(f.tag), show-value(f)))
      .flatten()
  ),
)

== Every field

#table(
  columns: (auto, auto, auto, 1fr),
  align: (left, left, right, left),
  table.header([*Tag*], [*IFD*], [*Count*], [*Value*]),
  ..fields
    .map(f => (raw(f.tag), f.ifd, str(f.count), show-value(f)))
    .flatten()
)
