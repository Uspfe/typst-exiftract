//! Generates the image fixtures used by the Rust and Typst test suites.
//!
//! Run with `cargo run --example mkfixtures` from the `plugin` directory.
//! The images are tiny solid-colour gradients; the point of them is the
//! metadata, which is written with kamadak-exif's own encoder so that the
//! fixtures stay in sync with the parser the plugin uses.

use std::fs;
use std::io::Cursor;
use std::path::Path;

use exif::experimental::Writer;
use exif::{Field, In, Rational, Tag, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/assets");
    fs::create_dir_all(&out)?;

    let pixels = encode_jpeg()?;
    fs::write(out.join("no-exif.jpg"), &pixels)?;
    fs::write(
        out.join("sample.jpg"),
        splice_jpeg_app1(&pixels, &exif_block()?),
    )?;
    fs::write(out.join("sample.png"), png_with_exif(&exif_block()?)?)?;
    fs::write(
        out.join("edge-cases.jpg"),
        splice_jpeg_app1(&pixels, &edge_case_block()?),
    )?;

    // The example document ships its own copy, so that it is self-contained
    // both in this repository and in the published package.
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples");
    fs::create_dir_all(&examples)?;
    fs::write(
        examples.join("photo.jpg"),
        splice_jpeg_app1(&pixels, &exif_block()?),
    )?;

    println!("wrote fixtures to {}", out.display());
    Ok(())
}

/// A representative Exif block: identification, capture settings, a date,
/// a GPS position and a multi-valued field.
fn exif_block() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let fields = vec![
        ascii(Tag::Make, "Typst"),
        ascii(Tag::Model, "Exif Fixture Camera"),
        ascii(Tag::Software, "typst-exif mkfixtures"),
        ascii(
            Tag::ImageDescription,
            "Fixture for the typst-exif test suite",
        ),
        Field {
            tag: Tag::Orientation,
            ifd_num: In::PRIMARY,
            value: Value::Short(vec![1]),
        },
        Field {
            tag: Tag::XResolution,
            ifd_num: In::PRIMARY,
            value: Value::Rational(vec![Rational { num: 72, denom: 1 }]),
        },
        Field {
            tag: Tag::YResolution,
            ifd_num: In::PRIMARY,
            value: Value::Rational(vec![Rational { num: 72, denom: 1 }]),
        },
        Field {
            tag: Tag::ResolutionUnit,
            ifd_num: In::PRIMARY,
            value: Value::Short(vec![2]),
        },
        ascii(Tag::DateTimeOriginal, "2024:05:17 09:30:00"),
        Field {
            tag: Tag::ExposureTime,
            ifd_num: In::PRIMARY,
            value: Value::Rational(vec![Rational { num: 1, denom: 200 }]),
        },
        Field {
            tag: Tag::FNumber,
            ifd_num: In::PRIMARY,
            value: Value::Rational(vec![Rational { num: 28, denom: 10 }]),
        },
        Field {
            tag: Tag::PhotographicSensitivity,
            ifd_num: In::PRIMARY,
            value: Value::Short(vec![400]),
        },
        Field {
            tag: Tag::FocalLength,
            ifd_num: In::PRIMARY,
            value: Value::Rational(vec![Rational { num: 35, denom: 1 }]),
        },
        Field {
            tag: Tag::PixelXDimension,
            ifd_num: In::PRIMARY,
            value: Value::Long(vec![16]),
        },
        Field {
            tag: Tag::PixelYDimension,
            ifd_num: In::PRIMARY,
            value: Value::Long(vec![16]),
        },
        ascii(Tag::GPSLatitudeRef, "N"),
        Field {
            tag: Tag::GPSLatitude,
            ifd_num: In::PRIMARY,
            value: Value::Rational(vec![
                Rational { num: 48, denom: 1 },
                Rational { num: 8, denom: 1 },
                Rational {
                    num: 4123,
                    denom: 100,
                },
            ]),
        },
        ascii(Tag::GPSLongitudeRef, "E"),
        Field {
            tag: Tag::GPSLongitude,
            ifd_num: In::PRIMARY,
            value: Value::Rational(vec![
                Rational { num: 11, denom: 1 },
                Rational { num: 34, denom: 1 },
                Rational {
                    num: 1157,
                    denom: 100,
                },
            ]),
        },
        Field {
            tag: Tag::GPSAltitudeRef,
            ifd_num: In::PRIMARY,
            value: Value::Byte(vec![0]),
        },
        Field {
            tag: Tag::GPSAltitude,
            ifd_num: In::PRIMARY,
            value: Value::Rational(vec![Rational {
                num: 5204,
                denom: 10,
            }]),
        },
        ascii(Tag::GPSDateStamp, "2024:05:17"),
        Field {
            tag: Tag::GPSTimeStamp,
            ifd_num: In::PRIMARY,
            value: Value::Rational(vec![
                Rational { num: 7, denom: 1 },
                Rational { num: 30, denom: 1 },
                // Fractional seconds: the reason GPSTimeStamp stays numbers.
                Rational {
                    num: 125,
                    denom: 10,
                },
            ]),
        },
        ascii(Tag::GPSImgDirectionRef, "T"),
        Field {
            tag: Tag::GPSImgDirection,
            ifd_num: In::PRIMARY,
            value: Value::Rational(vec![Rational {
                num: 2705,
                denom: 10,
            }]),
        },
        ascii(Tag::GPSSpeedRef, "K"),
        Field {
            tag: Tag::GPSSpeed,
            ifd_num: In::PRIMARY,
            value: Value::Rational(vec![Rational { num: 12, denom: 1 }]),
        },
        Field {
            tag: Tag::SubjectDistance,
            ifd_num: In::PRIMARY,
            value: Value::Rational(vec![Rational { num: 25, denom: 10 }]),
        },
        Field {
            tag: Tag::ExifVersion,
            ifd_num: In::PRIMARY,
            value: Value::Undefined(b"0232".to_vec(), 0),
        },
    ];

    let mut writer = Writer::new();
    for field in &fields {
        writer.push_field(field);
    }
    let mut buf = Cursor::new(Vec::new());
    writer.write(&mut buf, true)?;
    Ok(buf.into_inner())
}

/// Values that interpretation must leave alone or handle defensively: an
/// unknown tag, a `0/0` rational, dates that do not parse or do not exist, a
/// southern/western position, an altitude below sea level, and a second
/// resolution unit in the thumbnail directory.
fn edge_case_block() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let fields = vec![
        Field {
            tag: Tag(exif::Context::Tiff, 0x9999),
            ifd_num: In::PRIMARY,
            value: Value::Short(vec![7]),
        },
        Field {
            tag: Tag::ExposureTime,
            ifd_num: In::PRIMARY,
            value: Value::Rational(vec![Rational { num: 0, denom: 0 }]),
        },
        // The blank value Exif uses for an unset date, and a date that the
        // calendar does not have.
        ascii(Tag::DateTimeOriginal, "    :  :     :  :  "),
        ascii(Tag::DateTime, "2024:02:30 10:00:00"),
        ascii(Tag::GPSDateStamp, "not a date"),
        ascii(Tag::GPSLatitudeRef, "S"),
        Field {
            tag: Tag::GPSLatitude,
            ifd_num: In::PRIMARY,
            value: Value::Rational(vec![
                Rational { num: 33, denom: 1 },
                Rational { num: 51, denom: 1 },
                Rational { num: 24, denom: 1 },
            ]),
        },
        ascii(Tag::GPSLongitudeRef, "W"),
        Field {
            tag: Tag::GPSLongitude,
            ifd_num: In::PRIMARY,
            value: Value::Rational(vec![
                Rational { num: 70, denom: 1 },
                Rational { num: 39, denom: 1 },
                Rational { num: 18, denom: 1 },
            ]),
        },
        Field {
            tag: Tag::GPSAltitudeRef,
            ifd_num: In::PRIMARY,
            value: Value::Byte(vec![1]),
        },
        Field {
            tag: Tag::GPSAltitude,
            ifd_num: In::PRIMARY,
            value: Value::Rational(vec![Rational { num: 120, denom: 1 }]),
        },
        // Same tags in the thumbnail directory, with a different unit, so that
        // neighbour lookups have to stay within one directory.
        Field {
            tag: Tag::XResolution,
            ifd_num: In::THUMBNAIL,
            value: Value::Rational(vec![Rational { num: 300, denom: 1 }]),
        },
        Field {
            tag: Tag::ResolutionUnit,
            ifd_num: In::THUMBNAIL,
            value: Value::Short(vec![3]),
        },
        Field {
            tag: Tag::XResolution,
            ifd_num: In::PRIMARY,
            value: Value::Rational(vec![Rational { num: 72, denom: 1 }]),
        },
        Field {
            tag: Tag::ResolutionUnit,
            ifd_num: In::PRIMARY,
            value: Value::Short(vec![2]),
        },
    ];

    let mut writer = Writer::new();
    for field in &fields {
        writer.push_field(field);
    }
    let mut buf = Cursor::new(Vec::new());
    writer.write(&mut buf, true)?;
    Ok(buf.into_inner())
}

fn ascii(tag: Tag, text: &str) -> Field {
    Field {
        tag,
        ifd_num: In::PRIMARY,
        value: Value::Ascii(vec![text.as_bytes().to_vec()]),
    }
}

/// A 16x16 gradient, so the fixtures are real images and not just metadata.
fn pixels() -> Vec<u8> {
    let mut data = Vec::with_capacity(16 * 16 * 3);
    for y in 0..16u8 {
        for x in 0..16u8 {
            data.extend_from_slice(&[x * 16, y * 16, 128]);
        }
    }
    data
}

fn encode_jpeg() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90).encode(
        &pixels(),
        16,
        16,
        image::ExtendedColorType::Rgb8,
    )?;
    Ok(out)
}

/// Inserts an `APP1` Exif segment directly after the JPEG's `SOI` marker.
fn splice_jpeg_app1(jpeg: &[u8], exif: &[u8]) -> Vec<u8> {
    // The APP1 length field counts itself, the "Exif\0\0" header and the
    // Exif block, but not the marker.
    let segment_len = 2 + 6 + exif.len();
    assert!(
        segment_len <= u16::MAX as usize,
        "Exif block too large for APP1"
    );

    let mut out = Vec::with_capacity(jpeg.len() + segment_len + 2);
    out.extend_from_slice(&jpeg[..2]); // SOI
    out.extend_from_slice(&[0xff, 0xe1]);
    out.extend_from_slice(&(segment_len as u16).to_be_bytes());
    out.extend_from_slice(b"Exif\0\0");
    out.extend_from_slice(exif);
    out.extend_from_slice(&jpeg[2..]);
    out
}

/// Encodes a PNG and inserts an `eXIf` chunk before the first `IDAT`.
fn png_with_exif(exif: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png).write_image(
        &pixels(),
        16,
        16,
        image::ExtendedColorType::Rgb8,
    )?;

    let idat = find_chunk(&png, b"IDAT").ok_or("no IDAT chunk in encoded PNG")?;
    let mut chunk = Vec::new();
    chunk.extend_from_slice(&(exif.len() as u32).to_be_bytes());
    chunk.extend_from_slice(b"eXIf");
    chunk.extend_from_slice(exif);
    chunk.extend_from_slice(&crc32(&chunk[4..]).to_be_bytes());

    let mut out = Vec::with_capacity(png.len() + chunk.len());
    out.extend_from_slice(&png[..idat]);
    out.extend_from_slice(&chunk);
    out.extend_from_slice(&png[idat..]);
    Ok(out)
}

/// Returns the offset of the length field of the first chunk of this type.
fn find_chunk(png: &[u8], kind: &[u8; 4]) -> Option<usize> {
    let mut pos = 8; // signature
    while pos + 8 <= png.len() {
        let len = u32::from_be_bytes(png[pos..pos + 4].try_into().ok()?) as usize;
        if &png[pos + 4..pos + 8] == kind {
            return Some(pos);
        }
        pos += 12 + len;
    }
    None
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in data {
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & (!(crc & 1)).wrapping_add(1));
        }
    }
    !crc
}

use image::ImageEncoder;
