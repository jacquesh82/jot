use jot_core::models::NoteType;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum DetectError {
    #[error("input too short to detect type (got {0} bytes, need at least 16)")]
    TooShort(usize),
    #[error("unknown file format")]
    UnknownFormat,
    #[error("invalid UTF-8 text content")]
    InvalidUtf8,
}

pub fn detect(bytes: &[u8]) -> Result<NoteType, DetectError> {
    if bytes.len() < 16 {
        return Err(DetectError::TooShort(bytes.len()));
    }

    // --- Image ---
    if bytes[0..3] == [0xFF, 0xD8, 0xFF] {
        return Ok(NoteType::Image); // JPEG
    }
    if bytes[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
        return Ok(NoteType::Image); // PNG
    }
    if &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Ok(NoteType::Image); // WebP (before WAV)
    }
    if &bytes[0..4] == b"GIF8" {
        return Ok(NoteType::Image); // GIF
    }
    if bytes[0..2] == [0x42, 0x4D] {
        return Ok(NoteType::Image); // BMP
    }
    if bytes[0..4] == [0x49, 0x49, 0x2A, 0x00] || bytes[0..4] == [0x4D, 0x4D, 0x00, 0x2A] {
        return Ok(NoteType::Image); // TIFF LE / BE
    }
    if &bytes[4..8] == b"ftyp" {
        if &bytes[8..12] == b"heic" || &bytes[8..12] == b"heix" {
            return Ok(NoteType::Image); // HEIC
        }
        if &bytes[8..12] == b"avif" || &bytes[8..12] == b"avis" {
            return Ok(NoteType::Image); // AVIF
        }
    }
    if bytes[0..4] == [0x00, 0x00, 0x01, 0x00] {
        return Ok(NoteType::Image); // ICO
    }
    if &bytes[0..4] == b"%PDF" {
        return Ok(NoteType::Image); // PDF
    }

    // --- Audio ---
    if &bytes[0..4] == b"OggS" {
        return Ok(NoteType::Voice); // OGG/Opus
    }
    if &bytes[0..3] == b"ID3" {
        return Ok(NoteType::Voice); // MP3 (ID3)
    }
    if bytes[0] == 0xFF && (bytes[1] & 0xE0) == 0xE0 {
        return Ok(NoteType::Voice); // MP3 raw sync word
    }
    if &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
        return Ok(NoteType::Voice); // WAV
    }
    if &bytes[0..4] == b"fLaC" {
        return Ok(NoteType::Voice); // FLAC
    }
    if bytes[0..2] == [0xFF, 0xF1] || bytes[0..2] == [0xFF, 0xF9] {
        return Ok(NoteType::Voice); // AAC (ADTS)
    }
    if &bytes[4..8] == b"ftyp" && &bytes[8..12] == b"M4A " {
        return Ok(NoteType::Voice); // M4A
    }
    if &bytes[0..4] == b"FORM" && &bytes[8..12] == b"AIFF" {
        return Ok(NoteType::Voice); // AIFF
    }
    if bytes[0..4] == [0x1A, 0x45, 0xDF, 0xA3] {
        return Ok(NoteType::Voice); // WebM
    }
    if &bytes[0..5] == b"#!AMR" {
        return Ok(NoteType::Voice); // AMR
    }
    if bytes[0..4] == [0x30, 0x26, 0xB2, 0x75] {
        return Ok(NoteType::Voice); // WMA/ASF
    }

    // --- Text (UTF-8) ---
    // Require valid UTF-8 with no null bytes (null bytes indicate binary content)
    if let Ok(s) = std::str::from_utf8(bytes) {
        if !s.contains('\0') {
            return Ok(NoteType::Text);
        }
    }

    Err(DetectError::UnknownFormat)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jot_core::models::NoteType;

    fn pad(prefix: &[u8]) -> Vec<u8> {
        let mut v = prefix.to_vec();
        v.resize(40, 0x00);
        v
    }

    // --- Image ---

    #[test]
    fn detects_jpeg() {
        assert_eq!(detect(&pad(&[0xFF, 0xD8, 0xFF])).unwrap(), NoteType::Image);
    }

    #[test]
    fn detects_png() {
        assert_eq!(
            detect(&pad(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A])).unwrap(),
            NoteType::Image
        );
    }

    #[test]
    fn detects_webp() {
        let mut b = pad(b"RIFF");
        b[8..12].copy_from_slice(b"WEBP");
        assert_eq!(detect(&b).unwrap(), NoteType::Image);
    }

    #[test]
    fn detects_gif() {
        assert_eq!(detect(&pad(b"GIF8")).unwrap(), NoteType::Image);
    }

    #[test]
    fn detects_bmp() {
        assert_eq!(detect(&pad(&[0x42, 0x4D])).unwrap(), NoteType::Image);
    }

    #[test]
    fn detects_tiff_le() {
        assert_eq!(detect(&pad(&[0x49, 0x49, 0x2A, 0x00])).unwrap(), NoteType::Image);
    }

    #[test]
    fn detects_tiff_be() {
        assert_eq!(detect(&pad(&[0x4D, 0x4D, 0x00, 0x2A])).unwrap(), NoteType::Image);
    }

    #[test]
    fn detects_heic() {
        let mut b = vec![0u8; 40];
        b[4..8].copy_from_slice(b"ftyp");
        b[8..12].copy_from_slice(b"heic");
        assert_eq!(detect(&b).unwrap(), NoteType::Image);
    }

    #[test]
    fn detects_avif() {
        let mut b = vec![0u8; 40];
        b[4..8].copy_from_slice(b"ftyp");
        b[8..12].copy_from_slice(b"avif");
        assert_eq!(detect(&b).unwrap(), NoteType::Image);
    }

    #[test]
    fn detects_ico() {
        assert_eq!(detect(&pad(&[0x00, 0x00, 0x01, 0x00])).unwrap(), NoteType::Image);
    }

    #[test]
    fn detects_pdf() {
        assert_eq!(detect(&pad(b"%PDF")).unwrap(), NoteType::Image);
    }

    // --- Audio ---

    #[test]
    fn detects_ogg() {
        assert_eq!(detect(&pad(b"OggS")).unwrap(), NoteType::Voice);
    }

    #[test]
    fn detects_mp3_id3() {
        assert_eq!(detect(&pad(b"ID3")).unwrap(), NoteType::Voice);
    }

    #[test]
    fn detects_mp3_raw() {
        assert_eq!(detect(&pad(&[0xFF, 0xE0])).unwrap(), NoteType::Voice);
    }

    #[test]
    fn detects_wav() {
        let mut b = pad(b"RIFF");
        b[8..12].copy_from_slice(b"WAVE");
        assert_eq!(detect(&b).unwrap(), NoteType::Voice);
    }

    #[test]
    fn detects_flac() {
        assert_eq!(detect(&pad(b"fLaC")).unwrap(), NoteType::Voice);
    }

    #[test]
    fn detects_aac_f1() {
        assert_eq!(detect(&pad(&[0xFF, 0xF1])).unwrap(), NoteType::Voice);
    }

    #[test]
    fn detects_aac_f9() {
        assert_eq!(detect(&pad(&[0xFF, 0xF9])).unwrap(), NoteType::Voice);
    }

    #[test]
    fn detects_m4a() {
        let mut b = vec![0u8; 40];
        b[4..8].copy_from_slice(b"ftyp");
        b[8..12].copy_from_slice(b"M4A ");
        assert_eq!(detect(&b).unwrap(), NoteType::Voice);
    }

    #[test]
    fn detects_aiff() {
        let mut b = pad(b"FORM");
        b[8..12].copy_from_slice(b"AIFF");
        assert_eq!(detect(&b).unwrap(), NoteType::Voice);
    }

    #[test]
    fn detects_webm() {
        assert_eq!(detect(&pad(&[0x1A, 0x45, 0xDF, 0xA3])).unwrap(), NoteType::Voice);
    }

    #[test]
    fn detects_amr() {
        assert_eq!(detect(&pad(b"#!AMR")).unwrap(), NoteType::Voice);
    }

    #[test]
    fn detects_wma() {
        assert_eq!(detect(&pad(&[0x30, 0x26, 0xB2, 0x75])).unwrap(), NoteType::Voice);
    }

    // --- Collisions ---

    #[test]
    fn riff_webp_is_image_not_voice() {
        let mut b = pad(b"RIFF");
        b[8..12].copy_from_slice(b"WEBP");
        assert_eq!(detect(&b).unwrap(), NoteType::Image);
    }

    #[test]
    fn riff_wave_is_voice_not_image() {
        let mut b = pad(b"RIFF");
        b[8..12].copy_from_slice(b"WAVE");
        assert_eq!(detect(&b).unwrap(), NoteType::Voice);
    }

    #[test]
    fn ftyp_heic_is_image_not_voice() {
        let mut b = vec![0u8; 40];
        b[4..8].copy_from_slice(b"ftyp");
        b[8..12].copy_from_slice(b"heic");
        assert_eq!(detect(&b).unwrap(), NoteType::Image);
    }

    // --- Text and errors ---

    #[test]
    fn detects_utf8_text() {
        let content = b"Hello, world! This is a plain text note with enough bytes.";
        assert_eq!(detect(content).unwrap(), NoteType::Text);
    }

    #[test]
    fn too_short_returns_error() {
        let short = &[0u8; 10];
        assert_eq!(detect(short), Err(DetectError::TooShort(10)));
    }

    #[test]
    fn unknown_binary_returns_error() {
        let zeros = &[0u8; 40];
        assert_eq!(detect(zeros), Err(DetectError::UnknownFormat));
    }
}
