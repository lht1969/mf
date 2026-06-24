use crate::error::MfError;
use crate::platform;

pub struct Clipboard {
    provider: Box<dyn platform::ClipboardProvider>,
}

impl Clipboard {
    pub fn new() -> Result<Self, MfError> {
        Ok(Clipboard {
            provider: platform::create_clipboard(),
        })
    }

    pub fn read_text(&self) -> Result<String, MfError> {
        self.provider.read_text()
    }

    pub fn read_image(&self) -> Result<platform::ClipboardImage, MfError> {
        self.provider.read_image()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> Result<bool, MfError> {
        self.provider.is_empty()
    }

    #[allow(dead_code)]
    pub fn clear(&self) -> Result<(), MfError> {
        self.provider.clear()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(target_os = "linux", ignore)]
    fn test_clipboard_new() {
        let clip = Clipboard::new();
        assert!(clip.is_ok());
    }

    #[test]
    #[cfg_attr(target_os = "linux", ignore)]
    #[cfg(not(target_os = "linux"))]
    fn test_clipboard_roundtrip() {
        use arboard::Clipboard as ArboardClipboard;

        let clip = Clipboard::new().expect("Failed to create clipboard");
        let test_text = "mf-test-clipboard-roundtrip";

        let mut arboard_clip =
            ArboardClipboard::new().expect("Failed to open arboard clipboard");
        arboard_clip
            .set_text(test_text)
            .expect("Failed to set clipboard text");

        let read_back = clip.read_text().expect("Failed to read clipboard");
        assert!(read_back.contains(test_text), "Expected '{test_text}' in '{read_back}'");
    }

    #[test]
    #[cfg_attr(target_os = "linux", ignore)]
    fn test_clipboard_clear_does_not_panic() {
        let clip = Clipboard::new().expect("Failed to create clipboard");
        let _ = clip.clear(); // 仅验证不 panic
    }
}
