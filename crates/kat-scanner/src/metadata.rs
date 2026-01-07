use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::Path;

use anyhow::Result;
use flate2::read::ZlibDecoder;
use image::{ImageFormat, ImageReader};
use exif::{self, Tag, Value};
use quick_xml::events::Event;
use quick_xml::Reader as XmlReader;

const MAX_METADATA_ENTRIES: usize = 256;

#[derive(Debug, Clone, Copy, Default)]
pub struct MetadataOptions {
    pub deep: bool,
}

fn set_if_empty(slot: &mut Option<String>, value: &str) {
    if slot.is_none() {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            *slot = Some(trimmed.to_string());
        }
    }
}

fn promote_metadata_field(metadata: &mut ImageMetadata, key: &str, value: &str) {
    let promote = |slot: &mut Option<String>| set_if_empty(slot, value);
    match key {
        "exif.make" => promote(&mut metadata.device_make),
        "exif.model" => promote(&mut metadata.device_model),
        "exif.lens_model" => promote(&mut metadata.lens_model),
        "exif.lens_make" => promote(&mut metadata.lens_model),
        "exif.software" => promote(&mut metadata.content_creator),
        "exif.artist" => promote(&mut metadata.content_creator),
        "exif.copyright" => promote(&mut metadata.credit),
        "exif.description" => promote(&mut metadata.color_profile_name),
        "exif.color_space" => promote(&mut metadata.color_space),
        "exif.datetime_original" | "exif.datetime_digitized" | "exif.datetime" => {
            promote(&mut metadata.content_created)
        }
        "exif.aperture_value" => promote(&mut metadata.aperture_value),
        "exif.max_aperture_value" => promote(&mut metadata.max_aperture_value),
        "exif.exposure_time" => promote(&mut metadata.exposure_time),
        "exif.f_number" => promote(&mut metadata.f_number),
        "exif.focal_length" => promote(&mut metadata.focal_length),
        "exif.iso" => promote(&mut metadata.iso_speed),
        "exif.flash" => promote(&mut metadata.flash),
        "exif.exposure_program" => promote(&mut metadata.exposure_program),
        "exif.metering_mode" => promote(&mut metadata.metering_mode),
        "exif.white_balance" => promote(&mut metadata.white_balance),
        "exif.gps_latitude" => promote(&mut metadata.gps_latitude),
        "exif.gps_longitude" => promote(&mut metadata.gps_longitude),
        "exif.orientation" => promote(&mut metadata.orientation),
        _ => {
            if key.starts_with("iptc.byline") || key == "iptc.byline_title" {
                promote(&mut metadata.content_creator);
            } else if key == "iptc.credit" {
                promote(&mut metadata.credit);
            } else if key == "iptc.copyright" {
                promote(&mut metadata.credit);
            } else if key == "xmp.Description.Credit" {
                promote(&mut metadata.credit);
            } else if key.starts_with("xmp.Description.Creator") || key == "xmp.Creator" {
                promote(&mut metadata.content_creator);
            } else if key == "xmp.CreatorTool" || key == "xmp.Description.CreatorTool" {
                promote(&mut metadata.content_creator);
            } else if key == "xmp.Description.ProfileDescription" {
                promote(&mut metadata.color_profile_name);
            } else if key == "xmp.Description.CreateDate" || key == "xmp.CreateDate" {
                promote(&mut metadata.content_created);
            } else if key == "xmp.Description.DigitalSourceType"
                || key == "xmp.Description.DigitalSourceFileType"
            {
                promote(&mut metadata.flags.iter_mut().find(|_| false).map(|_| String::new()));
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ImageMetadata {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub color_type: Option<String>,
    pub has_alpha: Option<bool>,
    pub format: Option<String>,
    pub orientation: Option<String>,
    pub color_profile_name: Option<String>,
    pub color_space: Option<String>,
    pub content_created: Option<String>,
    pub device_make: Option<String>,
    pub device_model: Option<String>,
    pub lens_model: Option<String>,
    pub aperture_value: Option<String>,
    pub max_aperture_value: Option<String>,
    pub exposure_time: Option<String>,
    pub f_number: Option<String>,
    pub focal_length: Option<String>,
    pub iso_speed: Option<String>,
    pub flash: Option<String>,
    pub exposure_program: Option<String>,
    pub metering_mode: Option<String>,
    pub white_balance: Option<String>,
    pub content_creator: Option<String>,
    pub credit: Option<String>,
    pub gps_latitude: Option<String>,
    pub gps_longitude: Option<String>,
    pub metadata_total_bytes: Option<u64>,
    pub metadata_sizes: BTreeMap<String, u64>,
    pub metadata: BTreeMap<String, String>,
    pub flags: Vec<String>,
}

pub fn inspect(path: &Path, bytes: &[u8], options: MetadataOptions) -> Result<ImageMetadata> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    if extension == "svg" {
        return Ok(ImageMetadata {
            width: None,
            height: None,
            color_type: Some("vector".to_string()),
            has_alpha: Some(true),
            format: Some("SVG".to_string()),
            orientation: None,
            color_profile_name: None,
            color_space: None,
            content_created: None,
            device_make: None,
            device_model: None,
            lens_model: None,
            aperture_value: None,
            max_aperture_value: None,
            exposure_time: None,
            f_number: None,
            focal_length: None,
            iso_speed: None,
            flash: None,
            exposure_program: None,
            metering_mode: None,
            white_balance: None,
            content_creator: None,
            credit: None,
            gps_latitude: None,
            gps_longitude: None,
            metadata_total_bytes: None,
            metadata_sizes: BTreeMap::new(),
            metadata: BTreeMap::new(),
            flags: Vec::new(),
        });
    }

    let format_guess = ImageFormat::from_path(path).ok();
    
    // Try to get dimensions from bytes first
    let (width, height) = match ImageReader::new(Cursor::new(bytes)).with_guessed_format() {
        Ok(reader) => match reader.into_dimensions() {
            Ok(dims) => dims,
            Err(_) => (0, 0), // Fallback or could fail here? Let's just use 0,0 and process other metadata
        },
        Err(_) => (0, 0),
    };
    
    // If dimensions failed (0,0) and we really care, we could error, but let's proceed to extract metadata
    // Check if we should hard fail on dimensions? Original code did `with_context`.
    // But `inspect` might be called with a prefix that is too short for dimensions?
    // scanner.rs will ensure we have enough bytes (full file or large prefix). 
    // If we have full file, this should work.
    
    
    let mut metadata = ImageMetadata {
        width: if width > 0 { Some(width) } else { None },
        height: if height > 0 { Some(height) } else { None },
        color_type: None,
        has_alpha: format_guess
            .map(format_has_alpha)
            .or_else(|| guess_alpha_from_ext(extension.as_str())),
        format: format_guess
            .map(|fmt| format!("{fmt:?}"))
            .or_else(|| (!extension.is_empty()).then(|| extension.to_uppercase())),
        orientation: None,
        color_profile_name: None,
        color_space: None,
        content_created: None,
        device_make: None,
        device_model: None,
        lens_model: None,
        aperture_value: None,
        max_aperture_value: None,
        exposure_time: None,
        f_number: None,
        focal_length: None,
        iso_speed: None,
        flash: None,
        exposure_program: None,
        metering_mode: None,
        white_balance: None,
        content_creator: None,
        credit: None,
        gps_latitude: None,
        gps_longitude: None,
        metadata_total_bytes: None,
        metadata_sizes: BTreeMap::new(),
        metadata: BTreeMap::new(),
        flags: Vec::new(),
    };

    if options.deep {
         collect_exif(bytes, &mut metadata);
         collect_iptc(bytes, &mut metadata);

         match extension.as_str() {
             "gif" => collect_gif_comments(bytes, &mut metadata),
             "png" => {
                 collect_png_chunks(bytes, &mut metadata);
                 collect_xmp(bytes, &mut metadata);
             }
             "webp" => collect_webp_chunks(bytes, &mut metadata),
             _ => collect_xmp(bytes, &mut metadata),
         }

         derive_ai_flags(&mut metadata);
         if !metadata.metadata_sizes.is_empty() {
             let total = metadata
                 .metadata_sizes
                 .values()
                 .copied()
                 .sum::<u64>();
             if total > 0 {
                 metadata.metadata_total_bytes = Some(total);
             }
         }
    }

    Ok(metadata)
}

fn collect_iptc(bytes: &[u8], metadata: &mut ImageMetadata) {
    // IPTC data typically follows the Photoshop Image Resource Block (IRB) format
    // Look for the 8BIM signature followed by the IPTC block
    const PHOTOSHOP_IRB: &[u8] = b"8BIM";
    let mut offset = 0;
    let mut iptc_size = 0;

    while offset + 8 < bytes.len() {
        if &bytes[offset..offset + 4] == PHOTOSHOP_IRB {
            // Found Photoshop IRB, check for IPTC block
            if offset + 12 < bytes.len() {
                let block_id = u16::from_be_bytes([bytes[offset + 4], bytes[offset + 5]]);
                if block_id == 0x0404 {  // IPTC block ID
                    let data_size = bytes[offset + 6] as usize * 256 + bytes[offset + 7] as usize;
                    let data_start = offset + 8;
                    let data_end = data_start + data_size;
                    
                    if data_end <= bytes.len() {
                        parse_iptc_data(&bytes[data_start..data_end], metadata);
                        iptc_size += data_size;
                    }
                }
                offset += 8 + ((bytes[offset + 6] as usize) * 256 + bytes[offset + 7] as usize);
            } else {
                break;
            }
        } else {
            offset += 1;
        }
    }

    if iptc_size > 0 {
        add_size(&mut metadata.metadata_sizes, "iptc", iptc_size as u64);
    }
}

fn parse_iptc_data(data: &[u8], metadata: &mut ImageMetadata) {
    // Simple IPTC parser that extracts common fields
    let mut i = 0;
    while i + 5 < data.len() {
        if data[i] == 0x1C {
            let record = data[i + 1];
            let dataset = data[i + 2];
            let length = if data[i + 3] < 0x80 {
                data[i + 3] as usize
            } else {
                // Handle extended length (2-byte)
                if i + 5 < data.len() {
                    ((data[i + 3] as usize & 0x7F) << 8) | (data[i + 4] as usize)
                } else {
                    i += 1;
                    continue;
                }
            };

            let value_start = if data[i + 3] < 0x80 { i + 4 } else { i + 5 };
            let value_end = value_start + length;
            
            if value_end > data.len() {
                break;
            }

            let value = &data[value_start..value_end];
            let key = match (record, dataset) {
                (2, 5) => Some("iptc.object_name"),
                (2, 25) => Some("iptc.keywords"),
                (2, 80) => Some("iptc.byline"),
                (2, 85) => Some("iptc.byline_title"),
                (2, 90) => Some("iptc.city"),
                (2, 92) => Some("iptc.sublocation"),
                (2, 100) => Some("iptc.country_code"),
                (2, 105) => Some("iptc.headline"),
                (2, 110) => Some("iptc.credit"),
                (2, 115) => Some("iptc.source"),
                (2, 116) => Some("iptc.copyright"),
                (2, 122) => Some("iptc.caption_writer"),
                _ => None,
            };

            if let Some(key) = key {
                if let Ok(value_str) = String::from_utf8(value.to_vec()) {
                    insert_metadata(metadata, key.to_string(), value_str);
                }
            }

            i = value_end;
        } else {
            i += 1;
        }
    }
}

fn collect_gif_comments(bytes: &[u8], metadata: &mut ImageMetadata) {
    // GIF comments are stored in 0xFE blocks
    let mut offset = 0;
    let mut comments = Vec::new();
    
    while offset + 3 < bytes.len() {
        if bytes[offset] == 0x21 && bytes[offset + 1] == 0xFE {
            // Found comment extension
            let mut comment_start = offset + 2; // Skip extension introducer and label
            let mut comment_bytes = Vec::new();
            
            // Process sub-blocks
            while comment_start < bytes.len() {
                let block_size = bytes[comment_start] as usize;
                if block_size == 0 {
                    break; // End of comment blocks
                }
                
                let block_end = comment_start + 1 + block_size;
                if block_end > bytes.len() {
                    break;
                }
                
                comment_bytes.extend_from_slice(&bytes[comment_start + 1..block_end]);
                comment_start = block_end;
            }
            
            if !comment_bytes.is_empty() {
                if let Ok(comment) = String::from_utf8(comment_bytes) {
                    comments.push(comment);
                }
            }
            
            offset = comment_start + 1; // Move past the terminator
        } else {
            offset += 1;
        }
    }
    
    if !comments.is_empty() {
        let total_size: usize = comments.iter().map(|s| s.len()).sum();
        add_size(&mut metadata.metadata_sizes, "gif.comment", total_size as u64);
        
        for (i, comment) in comments.into_iter().enumerate() {
            if i < 5 { // Limit number of comments to prevent excessive output
                insert_metadata(
                    metadata, 
                    format!("gif.comment.{}", i + 1), 
                    comment.trim().to_string()
                );
            }
        }
    }
}

fn format_has_alpha(format: ImageFormat) -> bool {
    matches!(
        format,
        ImageFormat::Png | ImageFormat::Gif | ImageFormat::WebP | ImageFormat::Avif
    )
}

fn guess_alpha_from_ext(ext: &str) -> Option<bool> {
    match ext {
        "png" | "gif" | "webp" | "svg" => Some(true),
        "jpg" | "jpeg" => Some(false),
        _ => None,
    }
}

// read_prefix removed


fn collect_exif(bytes: &[u8], metadata: &mut ImageMetadata) {
    let mut cursor = Cursor::new(bytes);
    if let Ok(exif) = exif::Reader::new().read_from_container(&mut cursor) {
        let mut orientation = None;
        
        for field in exif.fields() {
            let tag = field.tag;
            let key = format!("exif.{}", exif_tag_key(tag));
            let display = format_exif_value(field, &exif);
            insert_metadata(metadata, key, display.clone());

            if orientation.is_none() && tag == Tag::Orientation {
                if let Some(orientation_value) = field.value.get_uint(0) {
                    orientation = orientation_label(orientation_value);
                }
            }
            if metadata.color_profile_name.is_none() && tag == Tag::ImageDescription {
                metadata.color_profile_name = Some(display);
            }
        }

        if metadata.orientation.is_none() {
            metadata.orientation = orientation;
        }

        if let Some(len) = estimate_exif_length(bytes) {
            add_size(&mut metadata.metadata_sizes, "exif", len as u64);
        }
    }
}

fn collect_xmp(bytes: &[u8], metadata: &mut ImageMetadata) {
    const START: &[u8] = br"<x:xmpmeta";
    const END: &[u8] = br"</x:xmpmeta>";

    if let Some(start) = find_slice(bytes, START) {
        let slice = &bytes[start..];
        if let Some(relative_end) = find_slice(slice, END) {
            let end = (relative_end + END.len()).min(slice.len());
            let packet = &slice[..end];
            add_size(
                &mut metadata.metadata_sizes,
                "xmp",
                packet.len() as u64,
            );
            if let Ok(xml) = std::str::from_utf8(packet) {
                flatten_xmp(xml, metadata);
            }
        }
    }
}

fn collect_png_chunks(bytes: &[u8], metadata: &mut ImageMetadata) {
    const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
    if bytes.len() < SIGNATURE.len() || &bytes[..SIGNATURE.len()] != SIGNATURE {
        return;
    }

    let mut offset = SIGNATURE.len();
    let mut chunks_parsed = 0;
    while offset + 12 <= bytes.len() && chunks_parsed < 128 {
        let length = u32::from_be_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        let chunk_start = offset + 4;
        let data_start = chunk_start + 4;
        let data_end = data_start.saturating_add(length);
        if data_end + 4 > bytes.len() {
            break;
        }

        let chunk_type = &bytes[chunk_start..chunk_start + 4];
        let chunk_data = &bytes[data_start..data_end];

        match chunk_type {
            b"iCCP" => parse_png_iccp(chunk_data, metadata),
            b"tEXt" => parse_png_text_chunk(chunk_data, metadata, false),
            b"zTXt" => parse_png_text_chunk(chunk_data, metadata, true),
            b"iTXt" => parse_png_itxt_chunk(chunk_data, metadata),
            b"eXIf" => parse_png_exif_chunk(chunk_data, metadata),
            _ => {}
        }

        offset = data_end + 4;
        chunks_parsed += 1;
        if chunk_type == b"IEND" {
            break;
        }
    }
}

fn collect_webp_chunks(bytes: &[u8], metadata: &mut ImageMetadata) {
    const RIFF: &[u8] = b"RIFF";
    const WEBP: &[u8] = b"WEBP";
    if bytes.len() < 12 || &bytes[..4] != RIFF || &bytes[8..12] != WEBP {
        return;
    }

    let mut offset = 12;
    let mut chunks = 0usize;
    while offset + 8 <= bytes.len() && chunks < 64 {
        let chunk_type = &bytes[offset..offset + 4];
        let chunk_len = u32::from_le_bytes([
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]) as usize;
        let data_start = offset + 8;
        let data_end = data_start.saturating_add(chunk_len);
        if data_end > bytes.len() {
            break;
        }

        let chunk_data = &bytes[data_start..data_end];
        match chunk_type {
            b"EXIF" => parse_webp_exif_chunk(chunk_data, metadata),
            b"XMP " => parse_webp_xmp_chunk(chunk_data, metadata),
            b"ICCP" => {
                add_size(&mut metadata.metadata_sizes, "icc", chunk_data.len() as u64);
                if metadata.color_profile_name.is_none() {
                    metadata.color_profile_name = Some("Embedded ICC profile".to_string());
                }
            }
            _ => {}
        }

        offset = data_end + (chunk_len % 2); // chunks are padded to even sizes
        chunks += 1;
    }
}

fn parse_png_iccp(chunk: &[u8], metadata: &mut ImageMetadata) {
    if let Some(pos) = chunk.iter().position(|b| *b == 0) {
        let name = String::from_utf8_lossy(&chunk[..pos]).trim().to_string();
        if !name.is_empty() {
            metadata.color_profile_name = Some(name);
        }
        if pos + 2 <= chunk.len() {
            let compressed = &chunk[pos + 2..];
            let mut decoder = ZlibDecoder::new(compressed);
            let mut decompressed = Vec::new();
            if decoder.read_to_end(&mut decompressed).is_ok() {
                add_size(
                    &mut metadata.metadata_sizes,
                    "icc",
                    decompressed.len() as u64,
                );
            }
        }
    }
}

fn parse_png_text_chunk(chunk: &[u8], metadata: &mut ImageMetadata, compressed: bool) {
    if let Some(pos) = chunk.iter().position(|b| *b == 0) {
        let keyword = match String::from_utf8(chunk[..pos].to_vec()) {
            Ok(text) => text,
            Err(_) => return,
        };

        let mut text_bytes = &chunk[pos + 1..];
        let mut buffer = Vec::new();
        if compressed {
            if text_bytes.is_empty() {
                return;
            }
            let compressed_payload = &text_bytes[1..];
            let mut decoder = ZlibDecoder::new(compressed_payload);
            if decoder.read_to_end(&mut buffer).is_err() {
                return;
            }
            text_bytes = &buffer;
        }

        if let Ok(value) = std::str::from_utf8(text_bytes) {
            add_size(
                &mut metadata.metadata_sizes,
                "png.text",
                text_bytes.len() as u64,
            );
            insert_metadata(
                metadata,
                format!("png.text.{keyword}"),
                value.trim().to_string(),
            );
        }
    }
}

fn parse_png_itxt_chunk(chunk: &[u8], metadata: &mut ImageMetadata) {
    let mut parts = chunk.split(|b| *b == 0);
    let keyword = match parts.next() {
        Some(bytes) => match String::from_utf8(bytes.to_vec()) {
            Ok(text) => text,
            Err(_) => return,
        },
        None => return,
    };

    let rest = parts.collect::<Vec<_>>();
    if rest.len() < 4 {
        return;
    }
    let compression_flag = rest[0].first().copied().unwrap_or(0);
    let compression_method = rest[1].first().copied().unwrap_or(0);
    let text_bytes = if compression_flag == 1 && compression_method == 0 {
        let mut decoder = ZlibDecoder::new(rest[3]);
        let mut buffer = Vec::new();
        if decoder.read_to_end(&mut buffer).is_err() {
            return;
        }
        buffer
    } else {
        rest[3].to_vec()
    };

    if let Ok(value) = String::from_utf8(text_bytes) {
        add_size(
            &mut metadata.metadata_sizes,
            "png.text",
            value.len() as u64,
        );
        insert_metadata(
            metadata,
            format!("png.text.{keyword}"),
            value.trim().to_string(),
        );
    }
}

fn parse_png_exif_chunk(chunk: &[u8], metadata: &mut ImageMetadata) {
    let mut buffer = Vec::with_capacity(chunk.len() + 6);
    buffer.extend_from_slice(b"Exif\0\0");
    buffer.extend_from_slice(chunk);
    collect_exif(&buffer, metadata);
    add_size(&mut metadata.metadata_sizes, "exif", chunk.len() as u64);
}

fn parse_webp_exif_chunk(chunk: &[u8], metadata: &mut ImageMetadata) {
    let mut buffer = Vec::with_capacity(chunk.len() + 6);
    buffer.extend_from_slice(b"Exif\0\0");
    buffer.extend_from_slice(chunk);
    collect_exif(&buffer, metadata);
    add_size(&mut metadata.metadata_sizes, "exif", chunk.len() as u64);
}

fn parse_webp_xmp_chunk(chunk: &[u8], metadata: &mut ImageMetadata) {
    add_size(&mut metadata.metadata_sizes, "xmp", chunk.len() as u64);
    if let Ok(xml) = std::str::from_utf8(chunk) {
        flatten_xmp(xml, metadata);
    }
}

fn estimate_exif_length(bytes: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i + 4 < bytes.len() {
        if bytes[i] == 0xFF && bytes[i + 1] == 0xE1 {
            let length = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
            if length < 2 {
                return None;
            }
            let data_len = length - 2;
            let start = i + 4;
            let end = start + data_len;
            if end <= bytes.len() && data_len >= 6 {
                if &bytes[start..start + 6] == b"Exif\0\0" {
                    return Some(data_len);
                }
            }
            i = end;
        } else {
            i += 1;
        }
    }
    None
}

fn insert_metadata(metadata: &mut ImageMetadata, key: String, value: String) {
    if key.is_empty() || value.is_empty() {
        return;
    }
    promote_metadata_field(metadata, &key, &value);
    if metadata.metadata.len() >= MAX_METADATA_ENTRIES {
        return;
    }
    metadata.metadata.insert(key, value);
}

fn add_size(sizes: &mut BTreeMap<String, u64>, key: &str, amount: u64) {
    let entry = sizes.entry(key.to_string()).or_insert(0);
    *entry += amount;
}

fn exif_tag_key(tag: Tag) -> String {
    match tag {
        Tag::Make => "make".into(),
        Tag::Model => "model".into(),
        Tag::Software => "software".into(),
        Tag::Artist => "artist".into(),
        Tag::Copyright => "copyright".into(),
        Tag::ExifVersion => "exif_version".into(),
        Tag::DateTimeOriginal => "datetime_original".into(),
        Tag::DateTimeDigitized => "datetime_digitized".into(),
        Tag::DateTime => "datetime".into(),
        Tag::ImageDescription => "image_description".into(),
        Tag::Orientation => "orientation".into(),
        Tag::XResolution => "x_resolution".into(),
        Tag::YResolution => "y_resolution".into(),
        Tag::ResolutionUnit => "resolution_unit".into(),
        Tag::YCbCrPositioning => "ycbcr_positioning".into(),
        Tag::ExifIFDPointer => "exif_ifd_pointer".into(),
        Tag::GPSInfoIFDPointer => "gps_info_ifd_pointer".into(),
        Tag::ExposureTime => "exposure_time".into(),
        Tag::FNumber => "f_number".into(),
        Tag::ExposureProgram => "exposure_program".into(),
        Tag::PhotographicSensitivity | Tag::ISOSpeed => "iso".into(),
        Tag::SensitivityType => "sensitivity_type".into(),
        Tag::RecommendedExposureIndex => "recommended_exposure_index".into(),
        Tag::OffsetTime => "offset_time".into(),
        Tag::OffsetTimeOriginal => "offset_time_original".into(),
        Tag::OffsetTimeDigitized => "offset_time_digitized".into(),
        Tag::ShutterSpeedValue => "shutter_speed".into(),
        Tag::ApertureValue => "aperture_value".into(),
        Tag::BrightnessValue => "brightness_value".into(),
        Tag::ExposureBiasValue => "exposure_bias".into(),
        Tag::MaxApertureValue => "max_aperture_value".into(),
        Tag::MeteringMode => "metering_mode".into(),
        Tag::Flash => "flash".into(),
        Tag::FocalLength => "focal_length".into(),
        Tag::FocalLengthIn35mmFilm => "focal_length_in_35mm".into(),
        Tag::MakerNote => "maker_note".into(),
        Tag::UserComment => "user_comment".into(),
        Tag::SubSecTime => "subsec_time".into(),
        Tag::SubSecTimeOriginal => "subsec_time_original".into(),
        Tag::SubSecTimeDigitized => "subsec_time_digitized".into(),
        Tag::FlashpixVersion => "flashpix_version".into(),
        Tag::ColorSpace => "color_space".into(),
        Tag::PixelXDimension => "pixel_x_dimension".into(),
        Tag::PixelYDimension => "pixel_y_dimension".into(),
        Tag::FocalPlaneXResolution => "focal_plane_x_resolution".into(),
        Tag::FocalPlaneYResolution => "focal_plane_y_resolution".into(),
        Tag::FocalPlaneResolutionUnit => "focal_plane_resolution_unit".into(),
        Tag::SensingMethod => "sensing_method".into(),
        Tag::FileSource => "file_source".into(),
        Tag::SceneType => "scene_type".into(),
        Tag::CustomRendered => "custom_rendered".into(),
        Tag::ExposureMode => "exposure_mode".into(),
        Tag::WhiteBalance => "white_balance".into(),
        Tag::DigitalZoomRatio => "digital_zoom_ratio".into(),
        Tag::SceneCaptureType => "scene_capture_type".into(),
        Tag::GainControl => "gain_control".into(),
        Tag::Contrast => "contrast".into(),
        Tag::Saturation => "saturation".into(),
        Tag::Sharpness => "sharpness".into(),
        Tag::SubjectDistanceRange => "subject_distance_range".into(),
        Tag::ImageUniqueID => "image_unique_id".into(),
        Tag::LensSpecification => "lens_specification".into(),
        Tag::LensMake => "lens_make".into(),
        Tag::LensModel => "lens_model".into(),
        Tag::LensSerialNumber => "lens_serial_number".into(),
        Tag::GPSVersionID => "gps_version".into(),
        Tag::GPSLatitudeRef => "gps_latitude_ref".into(),
        Tag::GPSLatitude => "gps_latitude".into(),
        Tag::GPSLongitudeRef => "gps_longitude_ref".into(),
        Tag::GPSLongitude => "gps_longitude".into(),
        Tag::GPSAltitudeRef => "gps_altitude_ref".into(),
        Tag::GPSAltitude => "gps_altitude".into(),
        Tag::GPSTimeStamp => "gps_timestamp".into(),
        Tag::GPSSpeedRef => "gps_speed_ref".into(),
        Tag::GPSSpeed => "gps_speed".into(),
        Tag::GPSImgDirectionRef => "gps_img_direction_ref".into(),
        Tag::GPSImgDirection => "gps_img_direction".into(),
        Tag::GPSDestBearingRef => "gps_dest_bearing_ref".into(),
        Tag::GPSDestBearing => "gps_dest_bearing".into(),
        Tag::GPSDateStamp => "gps_datestamp".into(),
        Tag::GPSDifferential => "gps_differential".into(),
        Tag::GPSHPositioningError => "gps_hpositioning_error".into(),
        _ => format!("{:?}", tag),
    }
}

fn format_exif_value(field: &exif::Field, ctx: &exif::Exif) -> String {
    match &field.value {
        Value::Ascii(arr) => arr
            .iter()
            .filter_map(|bytes| std::str::from_utf8(bytes).ok())
            .map(|s| s.trim_matches('\0'))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(", "),
        Value::Rational(nums) => nums
            .iter()
            .map(|rat| format!("{:.4}", rat.to_f64()))
            .collect::<Vec<_>>()
            .join(", "),
        Value::SRational(nums) => nums
            .iter()
            .map(|rat| format!("{:.4}", rat.to_f64()))
            .collect::<Vec<_>>()
            .join(", "),
        Value::Byte(bytes) => {
            if bytes.len() <= 8 {
                format!("{:?}", bytes)
            } else {
                format!("{} bytes", bytes.len())
            }
        }
        Value::Short(nums) => nums
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(", "),
        Value::Long(nums) => nums
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(", "),
        Value::SLong(nums) => nums
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(", "),
        Value::Float(nums) => nums
            .iter()
            .map(|n| format!("{n}"))
            .collect::<Vec<_>>()
            .join(", "),
        Value::Double(nums) => nums
            .iter()
            .map(|n| format!("{n}"))
            .collect::<Vec<_>>()
            .join(", "),
        _ => field.display_value().with_unit(ctx).to_string(),
    }
}

fn orientation_label(value: u32) -> Option<String> {
    match value {
        1 => Some("normal".to_string()),
        2 => Some("mirrored horizontal".to_string()),
        3 => Some("rotated 180".to_string()),
        4 => Some("mirrored vertical".to_string()),
        5 => Some("mirrored horizontal + rotated 270".to_string()),
        6 => Some("rotated 90".to_string()),
        7 => Some("mirrored horizontal + rotated 90".to_string()),
        8 => Some("rotated 270".to_string()),
        _ => None,
    }
}

fn find_slice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|window| window == needle)
}

fn flatten_xmp(xml: &str, metadata: &mut ImageMetadata) {
    let mut reader = XmlReader::from_str(xml);
    reader.trim_text(true);
    let mut buf = Vec::new();
    let mut path = Vec::new();
    let mut remaining = MAX_METADATA_ENTRIES.saturating_sub(metadata.metadata.len());

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let name = e.name().as_ref().to_vec();
                let tag = String::from_utf8_lossy(&name).to_string();
                let _is_empty = matches!(reader.read_event_into(&mut Vec::new()), Ok(Event::Empty(_)));
                path.push(tag.clone());
                
                for attr in e.attributes().flatten() {
                    if remaining == 0 {
                        break;
                    }
                    let attr_key = String::from_utf8_lossy(attr.key.as_ref());
                    
                    // Skip xmlns namespace declarations - they're not useful metadata
                    if attr_key.starts_with("xmlns") {
                        continue;
                    }
                    // Skip rdf:about="" which is always empty
                    if attr_key == "rdf:about" {
                        continue;
                    }
                    
                    if let Ok(value) = attr.unescape_value().map(|v| v.to_string()) {
                        // Skip empty values
                        if value.is_empty() {
                            continue;
                        }
                        
                        // Create a cleaner key for important XMP attributes
                        // e.g., "photoshop:Credit" -> "xmp.photoshop.Credit"
                        // e.g., "Iptc4xmpExt:DigitalSourceType" -> "xmp.Iptc4xmpExt.DigitalSourceType"
                        let key = if attr_key.contains(':') {
                            format!("xmp.{}", attr_key.replace(':', "."))
                        } else {
                            format!("xmp.{}.{}", path.join("."), attr_key)
                        };
                        
                        insert_metadata(metadata, key, value);
                        remaining = MAX_METADATA_ENTRIES.saturating_sub(metadata.metadata.len());
                    }
                }
                
                // For self-closing elements (Event::Empty), pop immediately
                if matches!(buf.last(), Some(_)) {
                    // Check if this was actually an empty element by peeking
                    // We handle this by checking the event type
                }
            }
            Ok(Event::Text(text)) => {
                if remaining == 0 {
                    continue;
                }
                let content = text.unescape().unwrap_or_default().trim().to_string();
                if !content.is_empty() {
                    let key = format!("xmp.{}", path.join("."));
                    insert_metadata(metadata, key, content);
                    remaining = MAX_METADATA_ENTRIES.saturating_sub(metadata.metadata.len());
                }
            }
            Ok(Event::End(_)) => {
                path.pop();
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
        if remaining == 0 {
            break;
        }
    }
}

fn derive_ai_flags(metadata: &mut ImageMetadata) {
    if metadata.metadata.is_empty() {
        return;
    }

    // AI Detection Signatures
    // -----------------------
    // We look for specific patterns in metadata values that indicate AI generation.
    // Sources: C2PA, Adobe Firefly, Midjourney, DALL-E, Stability AI, Google, etc.
    
    const AI_PATTERNS: &[(&str, &str)] = &[
        // Major AI generators
        ("midjourney", "ai_generated: midjourney"),
        ("stable diffusion", "ai_generated: stable_diffusion"),
        ("stability.ai", "ai_generated: stable_diffusion"),
        ("dall-e", "ai_generated: dalle"),
        ("dall·e", "ai_generated: dalle"),
        ("openai", "ai_generated: openai"),
        ("chatgpt", "ai_generated: openai"),
        ("adobe firefly", "ai_generated: adobe_firefly"),
        ("firefly", "ai_generated: adobe_firefly"),
        ("leonardo.ai", "ai_generated: leonardo"),
        ("runway", "ai_generated: runway"),
        ("pika", "ai_generated: pika"),
        ("ideogram", "ai_generated: ideogram"),
        ("flux", "ai_generated: flux"),
        
        // Google AI
        ("made with google ai", "ai_generated: google"),
        ("google ai", "ai_generated: google"),
        ("imagen", "ai_generated: google_imagen"),
        ("gemini", "ai_generated: google_gemini"),
        
        // Microsoft
        ("bing image creator", "ai_generated: bing"),
        ("microsoft copilot", "ai_generated: copilot"),
        ("designer.microsoft", "ai_generated: microsoft_designer"),
        
        // Local/open source tools
        ("comfyui", "ai_generated: comfyui"),
        ("automatic1111", "ai_generated: automatic1111"),
        ("invokeai", "ai_generated: invokeai"),
        
        // Generic AI indicators
        ("generative ai", "ai_generated: generic"),
        ("ai generated", "ai_generated: generic"),
        ("created using generative ai", "ai_generated: generic"),
        ("generated by ai", "ai_generated: generic"),
        
        // IPTC DigitalSourceType standard values
        ("trainedalgorithmicmedia", "ai_generated: iptc_certified"),
        ("compositewithtrainedalgorithmicmedia", "ai_generated: iptc_certified_composite"),
        ("algorithmicmedia", "ai_generated: algorithmic"),
        
        // Content provenance
        ("synthid", "watermark: synthid"),
        ("c2pa", "provenance: c2pa"),
        ("content credentials", "provenance: content_credentials"),
        ("contentauthenticity.org", "provenance: cai"),
    ];

    // Check values for AI patterns
    for value in metadata.metadata.values() {
        let value_lower = value.to_lowercase();
        for (needle, flag) in AI_PATTERNS {
            if value_lower.contains(needle) {
                metadata.flags.push((*flag).to_string());
            }
        }
    }

    // Check specific XMP/IPTC keys that indicate AI generation
    // These are the standard fields used by Google, Adobe, and others
    
    // IPTC DigitalSourceType - the official standard for marking AI images
    if let Some(source_type) = metadata.metadata.get("xmp.Iptc4xmpExt.DigitalSourceType") {
        let source_lower = source_type.to_lowercase();
        if source_lower.contains("trainedalgorithmicmedia") {
            metadata.flags.push("ai_generated: iptc_certified".to_string());
        } else if source_lower.contains("algorithmicmedia") {
            metadata.flags.push("ai_generated: algorithmic".to_string());
        }
    }
    
    // Photoshop Credit field - used by Google AI images
    if let Some(credit) = metadata.metadata.get("xmp.photoshop.Credit") {
        let credit_lower = credit.to_lowercase();
        if credit_lower.contains("google ai") || credit_lower.contains("made with") && credit_lower.contains("ai") {
            metadata.flags.push("ai_generated: google".to_string());
        }
        // Store the credit itself as a flag for visibility
        metadata.flags.push(format!("credit: {}", credit));
    }

    // Check keys for prompt/watermark indicators
    for key in metadata.metadata.keys() {
        let key_lower = key.to_lowercase();
        if key_lower.contains("watermark") {
            metadata.flags.push("embedded_watermark_metadata".to_string());
        }
        if key_lower.contains("prompt") && !key_lower.contains("copyright") {
            metadata.flags.push("contains_prompt_metadata".to_string());
        }
        if key_lower.contains("com.adobe.generative") {
            metadata.flags.push("ai_generated: adobe_firefly".to_string());
        }
        // Stable Diffusion / ComfyUI specific keys
        if key_lower.contains("parameters") || key_lower.contains("workflow") {
            if metadata.metadata.get(key).map(|v| v.len() > 50).unwrap_or(false) {
                metadata.flags.push("contains_generation_params".to_string());
            }
        }
    }

    // Special checks for specific software signatures in EXIF
    if let Some(software) = metadata.metadata.get("exif.software").map(|s| s.to_lowercase()) {
        if software.contains("midjourney") {
            metadata.flags.push("ai_generated: midjourney".to_string());
        }
        if software.contains("stable diffusion") || software.contains("comfyui") {
            metadata.flags.push("ai_generated: stable_diffusion".to_string());
        }
    }

    metadata.flags.sort();
    metadata.flags.dedup();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_ai_flags() {
        let mut metadata = ImageMetadata::default();
        
        // Case 1: Firefly via key pattern
        metadata.metadata.insert("xmp.com.adobe.generative".to_string(), "true".to_string());
        derive_ai_flags(&mut metadata);
        assert!(metadata.flags.contains(&"ai_generated: adobe_firefly".to_string()));
        
        // Case 2: Midjourney via Software
        metadata.metadata.clear();
        metadata.flags.clear();
        metadata.metadata.insert("exif.software".to_string(), "Midjourney 5.2".to_string());
        derive_ai_flags(&mut metadata);
        assert!(metadata.flags.contains(&"ai_generated: midjourney".to_string()));

        // Case 3: Prompt detection
        metadata.metadata.clear();
        metadata.flags.clear();
        metadata.metadata.insert("png.text.sd-prompt".to_string(), "a cat".to_string());
        derive_ai_flags(&mut metadata);
        assert!(metadata.flags.contains(&"contains_prompt_metadata".to_string()));
        
        // Case 4: Google AI via photoshop.Credit
        metadata.metadata.clear();
        metadata.flags.clear();
        metadata.metadata.insert("xmp.photoshop.Credit".to_string(), "Made with Google AI".to_string());
        derive_ai_flags(&mut metadata);
        assert!(metadata.flags.contains(&"ai_generated: google".to_string()));
        assert!(metadata.flags.contains(&"credit: Made with Google AI".to_string()));
        
        // Case 5: IPTC DigitalSourceType
        metadata.metadata.clear();
        metadata.flags.clear();
        metadata.metadata.insert("xmp.Iptc4xmpExt.DigitalSourceType".to_string(), 
            "http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia".to_string());
        derive_ai_flags(&mut metadata);
        assert!(metadata.flags.contains(&"ai_generated: iptc_certified".to_string()));
        
        // Case 6: DALL-E via value
        metadata.metadata.clear();
        metadata.flags.clear();
        metadata.metadata.insert("xmp.description".to_string(), "Generated by DALL-E 3".to_string());
        derive_ai_flags(&mut metadata);
        assert!(metadata.flags.contains(&"ai_generated: dalle".to_string()));
    }
}
