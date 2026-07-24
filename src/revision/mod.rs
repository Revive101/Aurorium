use std::fmt::Display;

#[derive(Debug, Default, Clone)]
pub struct Asset {
    pub file_name: String,
    pub file_type: String,
    pub size: i64,
    pub header_size: i64,
    pub compressed_header_size: i64,
    pub crc: u32,
    pub header_crc: u32,
}

#[derive(Debug, Clone)]
pub struct Revision {
    /// Revision name (e.g., `V_r773351.Wizard_1_570_0_Live`)
    pub name: String,

    /// Numeric revision number for comparison (e.g., `773351`)
    pub number: i64,
}

impl Display for Revision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
