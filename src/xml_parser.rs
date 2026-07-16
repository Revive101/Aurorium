use quick_xml::{Reader, events::Event};
use std::{fs::File, io::BufReader, path::Path};

use crate::{errors::XmlParseError, revision::asset_list::Asset};

enum Field {
    Src,
    FileType,
    Size,
    HeaderSize,
    CompressedHeaderSize,
    Crc,
    HeaderCrc,
}

pub fn parse_file_list<P>(path: P) -> miette::Result<(Vec<Asset>, Vec<Asset>)>
where
    P: AsRef<Path>,
{
    let file = File::open(path).map_err(XmlParseError::FileOpen)?;
    let reader = BufReader::with_capacity(1 << 20, file);
    let mut xml = Reader::from_reader(reader);
    xml.config_mut().trim_text(true);

    let mut buf: Vec<u8> = Vec::with_capacity(8 * 1024);

    let mut entries: Vec<Asset> = Vec::new();
    let mut current = Asset::default();

    let mut depth_skipped = 0u32;
    let mut in_record = false;
    let mut field: Option<Field> = None;

    loop {
        match xml.read_event_into(&mut buf).map_err(XmlParseError::Read)? {
            Event::Eof => break,

            Event::Start(e) => match e.local_name().as_ref() {
                b"_TableList" | b"About" => depth_skipped += 1,
                b"RECORD" if depth_skipped == 0 => {
                    in_record = true;
                    current = Asset::default();
                }
                b"SrcFileName" => field = Some(Field::Src),
                b"FileType" => field = Some(Field::FileType),
                b"Size" => field = Some(Field::Size),
                b"HeaderSize" => field = Some(Field::HeaderSize),
                b"CompressedHeaderSize" => field = Some(Field::CompressedHeaderSize),
                b"CRC" => field = Some(Field::Crc),
                b"HeaderCRC" => field = Some(Field::HeaderCrc),
                _ => {}
            },

            Event::Text(t) => {
                if let Some(f) = &field {
                    let text = t
                        .xml_content(quick_xml::XmlVersion::Implicit1_0)
                        .map_err(XmlParseError::Encoding)?;

                    match f {
                        Field::Src => current.file_name = text.into_owned(),
                        Field::FileType => current.file_type = text.parse().unwrap_or_default(),
                        Field::Size => current.size = text.parse().unwrap_or_default(),
                        Field::HeaderSize => current.header_size = text.parse().unwrap_or_default(),
                        Field::CompressedHeaderSize => {
                            current.compressed_header_size = text.parse().unwrap_or_default()
                        }
                        Field::Crc => current.crc = text.parse().unwrap_or_default(),
                        Field::HeaderCrc => current.header_crc = text.parse().unwrap_or_default(),
                    }
                }
            }

            Event::End(e) => match e.local_name().as_ref() {
                b"_TableList" | b"About" => depth_skipped = depth_skipped.saturating_sub(1),
                b"RECORD" => {
                    if in_record && depth_skipped == 0 {
                        entries.push(std::mem::take(&mut current));
                    }
                    in_record = false;
                }
                b"SrcFileName"
                | b"TarFileName"
                | b"FileType"
                | b"Size"
                | b"HeaderSize"
                | b"CompressedHeaderSize"
                | b"CRC"
                | b"HeaderCRC" => field = None,
                _ => {}
            },

            _ => {}
        }

        buf.clear();
    }

    let (wads, utils) = entries
        .into_iter()
        .partition(|f| f.file_name.ends_with(".wad"));

    Ok((wads, utils))
}
