use rust_embed::Embed;

#[derive(Embed)]
#[folder = "frontend/"]
pub struct Assets;

impl Assets {
    /// Get index.html content
    pub fn index_html() -> Option<String> {
        Self::get("index.html")
            .map(|f| String::from_utf8_lossy(f.data.as_ref()).to_string())
    }

    /// Get a specific asset by path
    pub fn get_asset(path: &str) -> Option<Vec<u8>> {
        Self::get(path).map(|f| f.data.as_ref().to_vec())
    }
}
