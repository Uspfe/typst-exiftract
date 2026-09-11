// Tests for the interpreting layer. Run with:
//   typst compile --root . --format pdf tests/test-interpret.typ /dev/null

#import "/lib.typ": read-exif, interpret

#let load(name) = interpret(read-exif(read("/tests/assets/" + name, encoding: none)))
#let fields = load("sample.jpg")
#let edge = load("edge-cases.jpg")

#let get(fields, name, ifd: "") = fields.find(f => f.tag == name and (ifd == "" or f.ifd == ifd))
#let value-of(fields, name) = get(fields, name).value
#let unit-of(fields, name) = get(fields, name).unit

// --- the shape of the result -----------------------------------------------

#assert.eq(fields.len(), 29)
#assert.eq(
  fields.first().keys().sorted(),
  ("count", "ifd", "number", "tag", "type", "unit", "value"),
)

// Interpretation only rewrites `value` and adds `unit`; the rest describes the
// file and is left as it was read.
#let raw = read-exif(read("/tests/assets/sample.jpg", encoding: none))
#assert.eq(
  fields.map(f => (f.tag, f.ifd, f.number, f.type, f.count)),
  raw.map(f => (f.tag, f.ifd, f.number, f.type, f.count)),
)

// --- rationals become floats -----------------------------------------------

#assert.eq(value-of(fields, "ExposureTime"), 0.005)
#assert.eq(value-of(fields, "FNumber"), 2.8)
#assert.eq(value-of(fields, "FocalLength"), 35.0)
#assert.eq(type(value-of(fields, "XResolution")), float)

// Integers and strings are already native and stay untouched.
#assert.eq(value-of(fields, "Orientation"), 1)
#assert.eq(value-of(fields, "Make"), "Typst")

// --- datetime --------------------------------------------------------------

#let taken = value-of(fields, "DateTimeOriginal")
#assert.eq(type(taken), datetime)
#assert.eq(taken.display("[year]-[month]-[day] [hour]:[minute]:[second]"), "2024-05-17 09:30:00")
#assert.eq(unit-of(fields, "DateTimeOriginal"), none)

// GPSDateStamp has no time of day.
#let stamped = value-of(fields, "GPSDateStamp")
#assert.eq(type(stamped), datetime)
#assert.eq(stamped.hour(), none)
#assert.eq(stamped.display("[year]-[month]-[day]"), "2024-05-17")

// --- angle -----------------------------------------------------------------

#assert.eq(type(value-of(fields, "GPSLatitude")), angle)
#assert.eq(value-of(fields, "GPSLatitude"), (48 + 8 / 60 + 41.23 / 3600) * 1deg)
#assert.eq(value-of(fields, "GPSLongitude"), (11 + 34 / 60 + 11.57 / 3600) * 1deg)
#assert.eq(value-of(fields, "GPSImgDirection"), 270.5deg)
#assert.eq(unit-of(fields, "GPSLatitude"), none)
#assert.eq(unit-of(fields, "GPSImgDirection"), none)

// The hemisphere comes from a neighbouring field and signs the angle.
#assert.eq(value-of(edge, "GPSLatitude"), -(33 + 51 / 60 + 24 / 3600) * 1deg)
#assert.eq(value-of(edge, "GPSLongitude"), -(70 + 39 / 60 + 18 / 3600) * 1deg)

// --- bytes -----------------------------------------------------------------

#let version = value-of(fields, "ExifVersion")
#assert.eq(type(version), bytes)
#assert.eq(str(version), "0232")

// --- units the specification fixes -----------------------------------------

#assert.eq(unit-of(fields, "ExposureTime"), "s")
#assert.eq(unit-of(fields, "FocalLength"), "mm")
#assert.eq(unit-of(fields, "SubjectDistance"), "m")
#assert.eq(unit-of(fields, "PixelXDimension"), "pixels")
#assert.eq(unit-of(fields, "GPSAltitude"), "m")
#assert.eq(unit-of(fields, "FNumber"), none)
#assert.eq(unit-of(fields, "Make"), none)

// --- units that come from a neighbouring field ------------------------------

#assert.eq(unit-of(fields, "XResolution"), "pixels per inch")
#assert.eq(unit-of(fields, "GPSSpeed"), "km/h")

// The neighbour must be the one in the same image directory.
#assert.eq(get(edge, "XResolution", ifd: "primary").unit, "pixels per inch")
#assert.eq(get(edge, "XResolution", ifd: "thumbnail").unit, "pixels per cm")

// --- signed by a neighbouring field ----------------------------------------

#assert.eq(value-of(fields, "GPSAltitude"), 520.4)
#assert.eq(value-of(edge, "GPSAltitude"), -120.0) // GPSAltitudeRef 1: below sea level

// --- what is deliberately not converted ------------------------------------

// GPSTimeStamp keeps fractional seconds, which neither `datetime` nor
// `duration` can hold.
#assert.eq(value-of(fields, "GPSTimeStamp"), (7.0, 30.0, 12.5))
#assert.eq(unit-of(fields, "GPSTimeStamp"), "h, min, s")

// Lengths stay numbers: Typst has no metre, so SubjectDistance could not
// follow FocalLength into a `length`.
#assert.eq(type(value-of(fields, "SubjectDistance")), float)
#assert.eq(type(value-of(fields, "FocalLength")), float)

// --- values that must not break anything -----------------------------------

// 0/0 is how Exif spells "unknown".
#assert(float.is-nan(value-of(edge, "ExposureTime")))

// Unparseable, blank and impossible dates stay strings rather than panicking.
#assert.eq(value-of(edge, "DateTimeOriginal"), "    :  :     :  :  ")
#assert.eq(value-of(edge, "DateTime"), "2024:02:30 10:00:00") // February has no 30th
#assert.eq(value-of(edge, "GPSDateStamp"), "not a date")

// An unknown tag is carried through untouched.
#assert.eq(value-of(edge, "tiff-0x9999"), 7)
#assert.eq(unit-of(edge, "tiff-0x9999"), none)

// --- composing with read-exif's own options --------------------------------

#assert.eq(interpret(()), ())
#assert.eq(
  interpret(read-exif(read("/tests/assets/no-exif.jpg", encoding: none), default: ())),
  (),
)

All interpret tests passed.
