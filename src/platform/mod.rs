use crate::error::MfError;

pub struct ClipboardImage {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

pub trait ClipboardProvider {
    fn read_text(&self) -> Result<String, MfError>;
    fn read_image(&self) -> Result<ClipboardImage, MfError>;
    fn is_empty(&self) -> Result<bool, MfError>;
    fn clear(&self) -> Result<(), MfError>;
}

pub fn create_clipboard() -> Box<dyn ClipboardProvider> {
    #[cfg(windows)]
    { Box::new(WindowsClipboard) }
    #[cfg(target_os = "linux")]
    { Box::new(LinuxClipboard) }
    #[cfg(target_os = "macos")]
    { Box::new(MacClipboard) }
}

#[cfg(windows)]
struct WindowsClipboard;
#[cfg(windows)]
impl ClipboardProvider for WindowsClipboard {
    fn read_text(&self) -> Result<String, MfError> {
        let mut cb = arboard::Clipboard::new()
            .map_err(|e| MfError::Clipboard(e.to_string()))?;
        cb.get_text()
            .map_err(|e| MfError::Clipboard(e.to_string()))
    }

    fn read_image(&self) -> Result<ClipboardImage, MfError> {
        let mut cb = arboard::Clipboard::new()
            .map_err(|e| MfError::Clipboard(e.to_string()))?;
        let img = cb
            .get()
            .image()
            .map_err(|e| MfError::Clipboard(e.to_string()))?;
        Ok(ClipboardImage {
            width: img.width,
            height: img.height,
            data: img.bytes.into_owned(),
        })
    }

    fn is_empty(&self) -> Result<bool, MfError> {
        let text = self.read_text()?;
        Ok(text.is_empty())
    }

    fn clear(&self) -> Result<(), MfError> {
        let mut cb = arboard::Clipboard::new()
            .map_err(|e| MfError::Clipboard(e.to_string()))?;
        cb.clear()
            .map_err(|e| MfError::Clipboard(e.to_string()))
    }
}

#[cfg(target_os = "linux")]
struct LinuxClipboard;
#[cfg(target_os = "linux")]
impl ClipboardProvider for LinuxClipboard {
    fn read_text(&self) -> Result<String, MfError> {
        Err(MfError::Clipboard(
            "Clipboard access requires xclip or wl-clipboard to be installed".into()
        ))
    }

    fn read_image(&self) -> Result<ClipboardImage, MfError> {
        Err(MfError::Clipboard(
            "Clipboard image access is not supported on Linux".into()
        ))
    }

    fn is_empty(&self) -> Result<bool, MfError> {
        Err(MfError::Clipboard(
            "Clipboard access requires xclip or wl-clipboard to be installed".into()
        ))
    }

    fn clear(&self) -> Result<(), MfError> {
        Err(MfError::Clipboard(
            "Clipboard access requires xclip or wl-clipboard to be installed".into()
        ))
    }
}

#[cfg(target_os = "macos")]
struct MacClipboard;
#[cfg(target_os = "macos")]
impl ClipboardProvider for MacClipboard {
    fn read_text(&self) -> Result<String, MfError> {
        let mut cb = arboard::Clipboard::new()
            .map_err(|e| MfError::Clipboard(e.to_string()))?;
        cb.get_text()
            .map_err(|e| MfError::Clipboard(e.to_string()))
    }

    fn read_image(&self) -> Result<ClipboardImage, MfError> {
        let mut cb = arboard::Clipboard::new()
            .map_err(|e| MfError::Clipboard(e.to_string()))?;
        let img = cb
            .get()
            .image()
            .map_err(|e| MfError::Clipboard(e.to_string()))?;
        Ok(ClipboardImage {
            width: img.width,
            height: img.height,
            data: img.bytes.into_owned(),
        })
    }

    fn is_empty(&self) -> Result<bool, MfError> {
        let text = self.read_text()?;
        Ok(text.is_empty())
    }

    fn clear(&self) -> Result<(), MfError> {
        let mut cb = arboard::Clipboard::new()
            .map_err(|e| MfError::Clipboard(e.to_string()))?;
        cb.clear()
            .map_err(|e| MfError::Clipboard(e.to_string()))
    }
}
