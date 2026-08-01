use std::fs;
use std::path::Path;

pub struct WorkspaceTools;

impl WorkspaceTools {
    pub fn write_file(path: &str, content: &str) -> Result<(), std::io::Error> {
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)
    }

    pub fn read_file(path: &str) -> Result<String, std::io::Error> {
        fs::read_to_string(path)
    }
}
