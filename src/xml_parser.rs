use crate::{errors::AssetFetcherError, models::asset::Asset};
use roxmltree::{Document, Node};

pub fn parse_xml(xml_content: &str) -> Result<(Vec<Asset>, Vec<Asset>), AssetFetcherError> {
    let doc = Document::parse(&xml_content)?;
    let root = doc.root_element();

    let mut records = Vec::new();

    for node in root.children().filter(|n| n.is_element()) {
        // Process each RECORD under the zone
        for inner_node in node.children().filter(|n| n.is_element() && n.tag_name().name() == "RECORD") {
            let record = extract_record_data(inner_node)?;
            records.push(record);
        }
    }

    cfg!(debug_assertions).then(|| {
        println!("Found {} records", records.len());
        for record in records.iter().take(5) {
            println!("Entry: {}", record.filename);
            println!("  Size: {} bytes", record.size);
            println!("");
        }
    });

    let (wads, utils) = records.into_iter().partition(|f| f.filename.ends_with(".wad"));

    Ok((wads, utils))
}

fn extract_record_data(record_node: Node) -> Result<Asset, AssetFetcherError> {
    let mut asset = Asset::default();

    // Map field names to their respective struct fields
    for child in record_node.children().filter(|n| n.is_element()) {
        let tag_name = child.tag_name().name();

        let text = child.text().unwrap_or("").trim();
        match tag_name {
            "SrcFileName" => asset.filename = text.to_string(),
            // "TarFileName" => record.tar_filename = text.to_string(),
            "FileType" => asset.file_type = text.to_string(),
            "Size" => asset.size = text.parse().unwrap_or(0),
            "HeaderSize" => asset.header_size = text.parse().unwrap_or(0),
            "CompressedHeaderSize" => asset.compressed_header_size = text.parse().unwrap_or(0),
            "CRC" => asset.crc = text.parse().unwrap_or(0),
            "HeaderCRC" => asset.header_crc = text.parse().unwrap_or(0),
            _ => {}
        }
    }

    Ok(asset)
}
