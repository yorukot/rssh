use crate::errors::RsshError;

#[derive(Debug)]
pub enum ClipboardContent {
    Text(String),
    Image(Vec<u8>),
    Empty,
}

pub fn read_clipboard() -> Result<ClipboardContent, RsshError> {
    let mut board = arboard::Clipboard::new().map_err(|e| RsshError::Clipboard(e.to_string()))?;

    if let Ok(img) = board.get_image() {
        let png_bytes = rgba_to_png(img.width, img.height, &img.bytes)?;
        return Ok(ClipboardContent::Image(png_bytes));
    }

    match board.get_text() {
        Ok(text) if !text.is_empty() => Ok(ClipboardContent::Text(text)),
        Ok(_) => Ok(ClipboardContent::Empty),
        Err(_) => Ok(ClipboardContent::Empty),
    }
}

fn rgba_to_png(width: usize, height: usize, rgba: &[u8]) -> Result<Vec<u8>, RsshError> {
    use std::io::Cursor;

    let mut buf = Vec::new();
    {
        let mut encoder = png::Encoder::new(Cursor::new(&mut buf), width as u32, height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|e| RsshError::Clipboard(format!("PNG encode error: {e}")))?;
        writer
            .write_image_data(rgba)
            .map_err(|e| RsshError::Clipboard(format!("PNG encode error: {e}")))?;
    }
    Ok(buf)
}
