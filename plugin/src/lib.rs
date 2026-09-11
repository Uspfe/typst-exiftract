//! WebAssembly plugin that extracts Exif metadata from an image and returns
//! it as JSON, for use from Typst through the plugin protocol.

use std::io::Cursor;

use exif::{Context, Field, Reader, Value};
use serde_json::{json, Map, Value as Json};
#[cfg(target_arch = "wasm32")]
use wasm_minimal_protocol::wasm_func;

#[cfg(target_arch = "wasm32")]
wasm_minimal_protocol::initiate_protocol!();

/// Knobs passed in from the Typst side as a small JSON object.
struct Options {
    /// Maximum number of elements kept for multi-valued fields.
    max_values: usize,
    /// Keep whatever could be parsed when the Exif block is damaged.
    lenient: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            max_values: 64,
            lenient: true,
        }
    }
}

impl Options {
    fn parse(raw: &[u8]) -> Options {
        let mut opts = Options::default();
        let Ok(Json::Object(map)) = serde_json::from_slice::<Json>(raw) else {
            return opts;
        };
        if let Some(n) = map.get("max_values").and_then(Json::as_u64) {
            opts.max_values = n as usize;
        }
        if let Some(b) = map.get("lenient").and_then(Json::as_bool) {
            opts.lenient = b;
        }
        opts
    }
}

/// The single entry point exposed to Typst.
///
/// `data` is the raw image file, `options` a JSON object (may be empty).
/// The answer is an envelope — `{"ok": true, "fields": [...]}` or
/// `{"ok": false, "error": "..."}` — rather than a protocol-level error, so
/// that the Typst side can decide whether a missing Exif block is fatal.
#[cfg_attr(target_arch = "wasm32", wasm_func)]
pub fn read_exif(data: &[u8], options: &[u8]) -> Vec<u8> {
    let opts = Options::parse(options);
    let json = match read(data, &opts) {
        Ok(fields) => json!({ "ok": true, "fields": fields }),
        Err(error) => json!({ "ok": false, "error": error }),
    };
    serde_json::to_vec(&json).unwrap_or_else(|e| {
        format!("{{\"ok\":false,\"error\":{}}}", Json::from(e.to_string())).into_bytes()
    })
}

/// Parses the Exif block and describes every field in it, in file order.
fn read(data: &[u8], opts: &Options) -> Result<Vec<Json>, String> {
    let mut reader = Reader::new();
    reader.continue_on_error(opts.lenient);

    let exif = reader
        .read_from_container(&mut Cursor::new(data))
        .or_else(|e| e.distill_partial_result(|_| ()))
        .map_err(|e| e.to_string())?;

    Ok(exif
        .fields()
        .map(|field| {
            let (value, kind, count) = encode_value(&field.value, opts.max_values);
            let mut record = Map::new();
            record.insert("tag".into(), tag_name(field).into());
            record.insert("ifd".into(), ifd_name(field).into());
            record.insert("number".into(), field.tag.number().into());
            record.insert("type".into(), kind.into());
            record.insert("count".into(), count.into());
            record.insert("value".into(), value);
            Json::Object(record)
        })
        .collect())
}

/// A stable, human-readable key for a field.
fn tag_name(field: &Field) -> String {
    match field.tag.description() {
        Some(_) => field.tag.to_string(),
        None => format!(
            "{}-0x{:04x}",
            context_name(field.tag.context()),
            field.tag.number()
        ),
    }
}

fn context_name(context: Context) -> &'static str {
    match context {
        Context::Tiff => "tiff",
        Context::Exif => "exif",
        Context::Gps => "gps",
        Context::Interop => "interop",
        _ => "other",
    }
}

/// The image directory a field belongs to: the TIFF IFD it lives in, or the
/// sub-IFD that gives it its meaning.
fn ifd_name(field: &Field) -> String {
    let index = field.ifd_num.index();
    match (field.tag.context(), index) {
        (Context::Tiff, 0) => "primary".into(),
        (Context::Tiff, 1) => "thumbnail".into(),
        (Context::Tiff, n) => format!("ifd{n}"),
        (context, 0) => context_name(context).into(),
        (context, n) => format!("{}@ifd{}", context_name(context), n),
    }
}

/// Converts an Exif value into JSON, as close to what the file holds as the
/// format allows: integers stay integers, rationals stay `[numerator,
/// denominator]` pairs, `UNDEFINED` stays a list of byte values.
///
/// Single-element fields — nearly all of them — are unwrapped to a bare
/// value. Returns the value, the Exif type name and the untruncated element
/// count.
fn encode_value(value: &Value, max: usize) -> (Json, &'static str, usize) {
    fn pack<T, F>(items: &[T], max: usize, f: F) -> (Json, usize)
    where
        F: Fn(&T) -> Json,
    {
        let count = items.len();
        match count {
            0 => (Json::Null, 0),
            1 => (f(&items[0]), 1),
            _ => (
                Json::Array(items[..count.min(max)].iter().map(f).collect()),
                count,
            ),
        }
    }

    /// JSON has no NaN or infinity; the Exif float types are unused in
    /// practice, but a hand-rolled file can still contain one.
    fn number(x: f64) -> Json {
        Json::from(x).as_f64().map_or(Json::Null, Json::from)
    }

    let (json, kind, count) = match value {
        Value::Byte(v) => {
            let (j, c) = pack(v, max, |x| (*x).into());
            (j, "byte", c)
        }
        Value::Ascii(v) => {
            let (j, c) = pack(v, max, |x| String::from_utf8_lossy(x).into_owned().into());
            (j, "ascii", c)
        }
        Value::Short(v) => {
            let (j, c) = pack(v, max, |x| (*x).into());
            (j, "short", c)
        }
        Value::Long(v) => {
            let (j, c) = pack(v, max, |x| (*x).into());
            (j, "long", c)
        }
        Value::Rational(v) => {
            let (j, c) = pack(v, max, |x| json!([x.num, x.denom]));
            (j, "rational", c)
        }
        Value::SByte(v) => {
            let (j, c) = pack(v, max, |x| (*x).into());
            (j, "sbyte", c)
        }
        Value::Undefined(v, _) => {
            let (j, c) = pack(v, max, |x| (*x).into());
            (j, "undefined", c)
        }
        Value::SShort(v) => {
            let (j, c) = pack(v, max, |x| (*x).into());
            (j, "sshort", c)
        }
        Value::SLong(v) => {
            let (j, c) = pack(v, max, |x| (*x).into());
            (j, "slong", c)
        }
        Value::SRational(v) => {
            let (j, c) = pack(v, max, |x| json!([x.num, x.denom]));
            (j, "srational", c)
        }
        Value::Float(v) => {
            let (j, c) = pack(v, max, |x| number(*x as f64));
            (j, "float", c)
        }
        Value::Double(v) => {
            let (j, c) = pack(v, max, |x| number(*x));
            (j, "double", c)
        }
        Value::Unknown(_, count, _) => (Json::Null, "unknown", *count as usize),
    };
    (json, kind, count)
}
