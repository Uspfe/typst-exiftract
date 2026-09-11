//! WebAssembly plugin that extracts Exif metadata from an image and returns
//! it as JSON, for use from Typst through the plugin protocol.

use std::io::Cursor;

use exif::{Context, Field, In, Reader, Value};
use serde_json::{json, Map, Value as Json};
#[cfg(target_arch = "wasm32")]
use wasm_minimal_protocol::wasm_func;

#[cfg(target_arch = "wasm32")]
wasm_minimal_protocol::initiate_protocol!();

/// Knobs passed in from the Typst side as a small JSON object.
struct Options {
    /// Maximum number of elements kept for multi-valued fields.
    max_values: usize,
    /// Maximum number of characters kept for `display` strings.
    max_display: usize,
    /// Keep whatever could be parsed when the Exif block is damaged.
    lenient: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            max_values: 64,
            max_display: 256,
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
        if let Some(n) = map.get("max_display").and_then(Json::as_u64) {
            opts.max_display = n as usize;
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
/// The answer is always valid JSON; failures are reported in the `ok` and
/// `error` members rather than through the protocol's error channel, so that
/// the Typst side can decide whether a missing Exif block is fatal.
#[cfg_attr(target_arch = "wasm32", wasm_func)]
pub fn read_exif(data: &[u8], options: &[u8]) -> Vec<u8> {
    let opts = Options::parse(options);
    let json = match read(data, &opts) {
        Ok(value) => value,
        Err(error) => json!({
            "ok": false,
            "error": error,
            "format": sniff(data),
        }),
    };
    serde_json::to_vec(&json).unwrap_or_else(|e| {
        format!("{{\"ok\":false,\"error\":{}}}", Json::from(e.to_string())).into_bytes()
    })
}

fn read(data: &[u8], opts: &Options) -> Result<Json, String> {
    let mut reader = Reader::new();
    reader.continue_on_error(opts.lenient);

    let mut warnings = Vec::new();
    let exif = reader
        .read_from_container(&mut Cursor::new(data))
        .or_else(|e| {
            e.distill_partial_result(|errors| {
                warnings.extend(errors.iter().map(|e| e.to_string()));
            })
        })
        .map_err(|e| e.to_string())?;

    let mut fields = Vec::new();
    let mut tags = Map::new();
    let mut display = Map::new();
    let mut ifds: Map<String, Json> = Map::new();

    // Two passes so that a field of the primary image always wins over the
    // same field of the embedded thumbnail in the flat `tags` map.
    let ordered = exif
        .fields()
        .filter(|f| f.ifd_num == In::PRIMARY)
        .chain(exif.fields().filter(|f| f.ifd_num != In::PRIMARY));

    for field in ordered {
        let name = tag_name(field);
        let group = group_name(field);
        let (value, kind, count, truncated) = encode_value(&field.value, opts.max_values);
        let shown = truncate(display_string(field, &exif), opts.max_display);

        let mut record = Map::new();
        record.insert("tag".into(), name.clone().into());
        record.insert("ifd".into(), group.clone().into());
        record.insert("number".into(), field.tag.number().into());
        record.insert("type".into(), kind.into());
        record.insert("count".into(), count.into());
        record.insert("value".into(), value.clone());
        record.insert("display".into(), shown.clone().into());
        if let Some(desc) = field.tag.description() {
            record.insert("description".into(), desc.into());
        }
        if truncated {
            record.insert("truncated".into(), true.into());
        }
        fields.push(Json::Object(record));

        tags.entry(name.clone()).or_insert_with(|| value.clone());
        display
            .entry(name.clone())
            .or_insert_with(|| shown.clone().into());
        match ifds
            .entry(group)
            .or_insert_with(|| Json::Object(Map::new()))
        {
            Json::Object(map) => {
                map.entry(name).or_insert(value);
            }
            _ => unreachable!(),
        }
    }

    Ok(json!({
        "ok": true,
        "format": sniff(data),
        "little-endian": exif.little_endian(),
        "count": fields.len(),
        "fields": fields,
        "tags": tags,
        "display": display,
        "ifds": ifds,
        "warnings": warnings,
    }))
}

/// The human-readable rendering of a field.
///
/// kamadak-exif quotes and escapes plain ASCII values, which reads badly in a
/// document. When a field got that default rendering we hand back the bare
/// text instead; fields with a dedicated formatter (units, enumerations) keep
/// the library's rendering.
fn display_string(field: &Field, exif: &exif::Exif) -> String {
    let shown = field.display_value().with_unit(exif).to_string();
    let Value::Ascii(parts) = &field.value else {
        return shown;
    };
    if shown == default_ascii_display(parts) {
        let texts: Vec<_> = parts
            .iter()
            .map(|p| String::from_utf8_lossy(p).into_owned())
            .collect();
        return texts.join(", ");
    }
    shown
}

/// Reproduces kamadak-exif's default rendering of an ASCII value.
fn default_ascii_display(parts: &[Vec<u8>]) -> String {
    let mut out = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push('"');
        for &c in part {
            match c {
                b'\\' | b'"' => {
                    out.push('\\');
                    out.push(c as char);
                }
                0x20..=0x7e => out.push(c as char),
                _ => out.push_str(&format!("\\x{c:02x}")),
            }
        }
        out.push('"');
    }
    out
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

/// The bucket a field is filed under in the `ifds` member: the TIFF IFD it
/// lives in, or the sub-IFD that gives it its meaning.
fn group_name(field: &Field) -> String {
    let index = field.ifd_num.index();
    match (field.tag.context(), index) {
        (Context::Tiff, 0) => "primary".into(),
        (Context::Tiff, 1) => "thumbnail".into(),
        (Context::Tiff, n) => format!("ifd{n}"),
        (context, 0) => context_name(context).into(),
        (context, n) => format!("{}@ifd{}", context_name(context), n),
    }
}

/// Converts an Exif value into JSON.
///
/// Single-element vectors — by far the common case — are unwrapped into a
/// bare scalar; the element count stays available in the `count` member.
/// Returns the value, its Exif type name, its untruncated element count and
/// whether elements were dropped to honour `max`.
fn encode_value(value: &Value, max: usize) -> (Json, &'static str, usize, bool) {
    fn pack<T, F>(items: &[T], max: usize, f: F) -> (Json, usize, bool)
    where
        F: Fn(&T) -> Json,
    {
        let count = items.len();
        let kept = count.min(max);
        match count {
            0 => (Json::Null, 0, false),
            1 => (f(&items[0]), 1, false),
            _ => (
                Json::Array(items[..kept].iter().map(f).collect()),
                count,
                kept < count,
            ),
        }
    }

    fn number(x: f64) -> Json {
        Json::from(x).as_f64().map_or(Json::Null, Json::from)
    }

    let (json, kind, count, truncated) = match value {
        Value::Byte(v) => {
            let (j, c, t) = pack(v, max, |x| (*x).into());
            (j, "byte", c, t)
        }
        Value::Ascii(v) => {
            let (j, c, t) = pack(v, max, |x| String::from_utf8_lossy(x).into_owned().into());
            (j, "ascii", c, t)
        }
        Value::Short(v) => {
            let (j, c, t) = pack(v, max, |x| (*x).into());
            (j, "short", c, t)
        }
        Value::Long(v) => {
            let (j, c, t) = pack(v, max, |x| (*x).into());
            (j, "long", c, t)
        }
        Value::Rational(v) => {
            let (j, c, t) = pack(v, max, |x| number(x.to_f64()));
            (j, "rational", c, t)
        }
        Value::SByte(v) => {
            let (j, c, t) = pack(v, max, |x| (*x).into());
            (j, "sbyte", c, t)
        }
        Value::Undefined(v, _) => {
            let (j, c, t) = pack(v, max, |x| (*x).into());
            (j, "undefined", c, t)
        }
        Value::SShort(v) => {
            let (j, c, t) = pack(v, max, |x| (*x).into());
            (j, "sshort", c, t)
        }
        Value::SLong(v) => {
            let (j, c, t) = pack(v, max, |x| (*x).into());
            (j, "slong", c, t)
        }
        Value::SRational(v) => {
            let (j, c, t) = pack(v, max, |x| number(x.to_f64()));
            (j, "srational", c, t)
        }
        Value::Float(v) => {
            let (j, c, t) = pack(v, max, |x| number(*x as f64));
            (j, "float", c, t)
        }
        Value::Double(v) => {
            let (j, c, t) = pack(v, max, |x| number(*x));
            (j, "double", c, t)
        }
        Value::Unknown(_, count, _) => (Json::Null, "unknown", *count as usize, false),
    };
    (json, kind, count, truncated)
}

fn truncate(mut text: String, max: usize) -> String {
    if text.chars().count() > max {
        let end = text.char_indices().nth(max).map_or(text.len(), |(i, _)| i);
        text.truncate(end);
        text.push('…');
    }
    text
}

/// Best-effort container detection, reported back for diagnostics.
fn sniff(data: &[u8]) -> &'static str {
    const TIFF_LE: &[u8] = b"II*\0";
    const TIFF_BE: &[u8] = b"MM\0*";
    if data.starts_with(&[0xff, 0xd8]) {
        "jpeg"
    } else if data.starts_with(TIFF_LE) || data.starts_with(TIFF_BE) {
        "tiff"
    } else if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        "png"
    } else if data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WEBP") {
        "webp"
    } else if data.get(4..8) == Some(b"ftyp") {
        "heif"
    } else {
        "unknown"
    }
}
