// typst-exif — read Exif metadata from images.
//
// The parsing happens in a WebAssembly plugin built from `plugin/`, which
// reads the image's Exif block and hands back JSON.

#let _plugin = plugin("exif.wasm")

/// Reads the Exif metadata of an image.
///
/// Returns the fields in the order the file stores them, one dictionary each:
///
/// ```typc
/// (
///   tag: "ExposureTime",  // tag name, or e.g. "exif-0x9999" if unknown
///   ifd: "exif",          // primary, thumbnail, exif, gps or interop
///   number: 33434,        // numeric tag id
///   type: "rational",     // Exif type of the value
///   count: 1,             // number of elements, before truncation
///   value: (1, 200),      // the value as the file stores it
/// )
/// ```
///
/// - data (bytes): The raw image file, for instance
///   `read("photo.jpg", encoding: none)`. JPEG, TIFF, PNG, WebP and
///   HEIF/HEIC/AVIF containers are understood.
/// - default (any): What to return when the image carries no readable Exif
///   metadata. Left at `auto`, that case panics instead.
/// - max-values (int): How many elements of a multi-valued field to keep.
///   Guards against fields such as `MakerNote`, which can be tens of
///   kilobytes. The untruncated length stays available as `count`.
/// - lenient (bool): Keep whatever could be parsed from a damaged Exif
///   block, instead of treating the damage as an error.
///
/// -> array
#let read-exif(
  data,
  default: auto,
  max-values: 64,
  lenient: true,
) = {
  if type(data) != bytes {
    panic(
      "read-exif expects the image data as bytes, found "
        + str(type(data))
        + "; load the file with `read(\"photo.jpg\", encoding: none)`",
    )
  }
  if type(max-values) != int or max-values < 0 {
    panic("max-values must be a non-negative integer")
  }
  if type(lenient) != bool {
    panic("lenient must be a boolean")
  }

  let options = bytes(json.encode((
    max_values: max-values,
    lenient: lenient,
  )))

  let result = json(_plugin.read_exif(data, options))

  if not result.ok {
    if default == auto {
      panic("could not read Exif metadata: " + result.error)
    }
    return default
  }

  result.fields
}
