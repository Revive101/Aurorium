use crate::models::asset::Asset;
use roxmltree::{Document, Node};
use std::fmt::Write;

pub fn parse_xml(xml_content: &str) -> anyhow::Result<(Vec<Asset>, Vec<Asset>)> {
    let doc = Document::parse(xml_content)?;
    let root = doc.root_element();

    let mut records = Vec::new();

    for node in root.children().filter(Node::is_element) {
        // Process each RECORD under the zone
        for inner_node in node.children().filter(|n| n.is_element() && n.tag_name().name() == "RECORD") {
            let record = extract_record_data(inner_node);
            records.push(record);
        }
    }

    cfg!(debug_assertions).then(|| {
        println!("Found {} records", records.len());
    });

    let (wads, utils) = records.into_iter().partition(|f| f.filename.ends_with(".wad"));

    Ok((wads, utils))
}

fn extract_record_data(record_node: Node) -> Asset {
    let mut asset = Asset::default();

    // Map field names to their respective struct fields
    for child in record_node.children().filter(Node::is_element) {
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

    asset
}

// ts looks so ugly like fr 🥀🥀
///////////////////////////////////////
pub async fn sanitize_content(text: &str) -> anyhow::Result<String> {
    println!("Sanitizing XML...");

    let doc = Document::parse(text)?;
    let root = doc.root_element();

    let mut output = String::new();
    output.push_str("<?xml version=\"1.0\" ?>\n<LatestFileList>\n");

    for child in root.children() {
        if child.is_element() && !matches!(child.tag_name().name(), "_TableList" | "About") {
            output.push_str(&node_to_string(child));
            output.push('\n');
        }
    }

    output.push_str("</LatestFileList>");
    Ok(output)
}

fn node_to_string(node: Node) -> String {
    let mut s = String::new();
    write!(s, "<{}>", node.tag_name().name()).unwrap();

    for child in node.children() {
        match () {
            () if child.is_element() => s.push_str(&node_to_string(child)),
            () if child.is_text() => s.push_str(child.text().unwrap_or("")),
            () => (),
        }
    }

    write!(s, "</{}>", node.tag_name().name()).unwrap();
    s
}
//////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::parse_xml;

    #[test]
    fn test_parse_xml() {
        let xml_content = r#"
            <LatestFileList>
                <Zafaria-ZF_Z13_Mirror_Lake_Drained>
                    <RECORD>
                        <SrcFileName>Data/GameData/Zafaria-ZF_Z13_Mirror_Lake_Drained.wad</SrcFileName>
                        <TarFileName></TarFileName>
                        <FileType>3</FileType>
                        <Size>410066</Size>
                        <HeaderSize>993</HeaderSize>
                        <CompressedHeaderSize>699</CompressedHeaderSize>
                        <CRC>3587946300</CRC>
                        <HeaderCRC>3214041276</HeaderCRC>
                    </RECORD>
                </Zafaria-ZF_Z13_Mirror_Lake_Drained>
                <_Shared-WorldData>
                    <RECORD>
                        <SrcFileName>Data/GameData/_Shared-WorldData.wad</SrcFileName>
                        <TarFileName></TarFileName>
                        <FileType>3</FileType>
                        <Size>503228191</Size>
                        <HeaderSize>1821164</HeaderSize>
                        <CompressedHeaderSize>583258</CompressedHeaderSize>
                        <CRC>10566522</CRC>
                        <HeaderCRC>1108099465</HeaderCRC>
                    </RECORD>
                </_Shared-WorldData>
            </LatestFileList>
        "#;

        let (wads, utils) = parse_xml(xml_content).unwrap();
        assert_eq!(wads.len(), 2);
        assert_eq!(utils.len(), 0);
        assert_eq!(wads[0].filename, "Data/GameData/Zafaria-ZF_Z13_Mirror_Lake_Drained.wad");
        assert_eq!(wads[1].filename, "Data/GameData/_Shared-WorldData.wad");
    }
}
