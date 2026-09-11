// A small report built from an image's Exif metadata.
//
//   typst compile --root .. examples/metadata.typ

#import "/lib.typ": read-exif

#set page(width: 15cm, height: auto, margin: 1.5cm)
#set text(size: 10pt)

#let photo = read("/tests/assets/sample.jpg", encoding: none)
#let data = read-exif(photo)

= #data.display.at("ImageDescription", default: "Untitled")

#grid(
  columns: (auto, 1fr),
  column-gutter: 1em,
  image(bytes(photo), width: 3cm),
  table(
    columns: 2,
    stroke: none,
    align: (right, left),
    ..(
      ("Camera", data.display.at("Make", default: "—") + " " + data.display.at("Model", default: "")),
      ("Taken", data.display.at("DateTimeOriginal", default: "—")),
      ("Exposure", data.display.at("ExposureTime", default: "—")),
      ("Aperture", data.display.at("FNumber", default: "—")),
      ("Focal length", data.display.at("FocalLength", default: "—")),
      ("Position", data.display.at("GPSLatitude", default: "—")
        + ", " + data.display.at("GPSLongitude", default: "—")),
    )
      .map(((label, value)) => (strong(label), value))
      .flatten()
  ),
)

== Every field

#table(
  columns: (auto, auto, 1fr),
  align: (left, left, left),
  table.header([*Tag*], [*IFD*], [*Value*]),
  ..data.fields.map(field => (raw(field.tag), field.ifd, field.display)).flatten()
)
