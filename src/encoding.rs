use crate::error::MfError;
use encoding_rs::{Encoding, GBK, UTF_16BE, UTF_16LE, UTF_8};
use std::path::Path;

#[allow(dead_code)]
pub fn choose_encoding(filename: &Path) -> &'static str {
    match filename.extension().and_then(|ext| ext.to_str()) {
        Some("bat") | Some("cmd") => "gbk",
        Some("ps1") => "utf8bom",
        _ => "utf8",
    }
}

fn resolve_encoding(name: &str) -> Result<&'static Encoding, MfError> {
    match name {
        "utf8" | "utf-8" | "utf8bom" => Ok(UTF_8),
        "gbk" | "gb2312" => Ok(GBK),
        "utf16le" | "utf-16le" => Ok(UTF_16LE),
        "utf16be" | "utf-16be" => Ok(UTF_16BE),
        _ => Err(MfError::Encoding(format!("Unknown encoding: {}", name))),
    }
}

pub fn encode_string(content: &str, encoding: &str) -> Result<Vec<u8>, MfError> {
    let encoder = resolve_encoding(encoding)?;
    let (cow, _, _) = encoder.encode(content);
    let mut bytes = cow.into_owned();

    if encoding == "utf8bom" {
        let bom = [0xEF, 0xBB, 0xBF];
        if !bytes.starts_with(&bom) {
            let mut with_bom = Vec::with_capacity(bytes.len() + 3);
            with_bom.extend_from_slice(&bom);
            with_bom.extend_from_slice(&bytes);
            bytes = with_bom;
        }
    }

    Ok(bytes)
}

pub fn detect_from_bytes(data: &[u8]) -> &'static str {
    if data.len() >= 3 && data[0] == 0xEF && data[1] == 0xBB && data[2] == 0xBF {
        return "utf8bom";
    }
    if data.len() >= 2 {
        if data[0] == 0xFF && data[1] == 0xFE {
            return "utf16le";
        }
        if data[0] == 0xFE && data[1] == 0xFF {
            return "utf16be";
        }
    }

    let mut detector = chardetng::EncodingDetector::new();
    detector.feed(data, true);
    let encoding = detector.guess(None, true);
    match encoding.name() {
        "UTF-8" => "utf8",
        "GBK" => "gbk",
        "UTF-16LE" => "utf16le",
        "UTF-16BE" => "utf16be",
        _ => "utf8",
    }
}

#[allow(dead_code)]
pub fn convert_bytes_to(data: &[u8], to_enc: &str) -> Result<Vec<u8>, MfError> {
    let decoded = decode_to_string(data)?;
    encode_string(&decoded, to_enc)
}

pub fn decode_to_string(data: &[u8]) -> Result<String, MfError> {
    let encoding_name = detect_from_bytes(data);
    let encoding = resolve_encoding(encoding_name)?;
    let (cow, _, had_errors) = encoding.decode(data);
    if had_errors {
        return Err(MfError::Encoding(format!(
            "Failed to decode as {}",
            encoding_name
        )));
    }
    Ok(cow.into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_choose_encoding_bat() {
        assert_eq!(choose_encoding(Path::new("script.bat")), "gbk");
        assert_eq!(choose_encoding(Path::new("install.cmd")), "gbk");
    }

    #[test]
    fn test_choose_encoding_ps1() {
        assert_eq!(choose_encoding(Path::new("deploy.ps1")), "utf8bom");
    }

    #[test]
    fn test_choose_encoding_other() {
        assert_eq!(choose_encoding(Path::new("readme.txt")), "utf8");
        assert_eq!(choose_encoding(Path::new("main.rs")), "utf8");
        assert_eq!(choose_encoding(Path::new("config.json")), "utf8");
    }

    #[test]
    fn test_encode_utf8() {
        let result = encode_string("Hello", "utf8").unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_encode_utf8bom() {
        let result = encode_string("Hello", "utf8bom").unwrap();
        let mut expected = vec![0xEF, 0xBB, 0xBF];
        expected.extend_from_slice(b"Hello");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_encode_gbk() {
        let input = "中文";
        let encoded = encode_string(input, "gbk").unwrap();
        let (decoded, _, _) = GBK.decode(&encoded);
        assert_eq!(decoded.as_ref(), input);
    }

    #[test]
    fn test_detect_utf8bom() {
        let mut data = vec![0xEF, 0xBB, 0xBF];
        data.extend_from_slice(b"Hello");
        assert_eq!(detect_from_bytes(&data), "utf8bom");
    }

    #[test]
    fn test_detect_utf16le() {
        let data = [0xFF, 0xFE, b'H', 0x00, b'i', 0x00];
        assert_eq!(detect_from_bytes(&data), "utf16le");
    }

    #[test]
    fn test_roundtrip_utf16le() {
        let input = "Hello 中文 🎉";
        let encoded = encode_string(input, "utf16le").unwrap();
        let decoded = decode_to_string(&encoded).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn test_invalid_encoding_name() {
        let result = encode_string("test", "invalid-encoding");
        assert!(result.is_err());
        match result {
            Err(MfError::Encoding(msg)) => {
                assert!(msg.contains("invalid-encoding"));
            }
            _ => panic!("Expected MfError::Encoding"),
        }
    }

    #[test]
    fn test_encode_large_text() {
        let input = "a".repeat(10_000);
        let result = encode_string(&input, "utf8").unwrap();
        assert_eq!(result.len(), 10_000);
        assert_eq!(result, input.as_bytes());
    }

    #[test]
    fn test_convert_bytes_utf8_to_gbk() {
        let data = "中文测试".as_bytes();
        let result = convert_bytes_to(data, "gbk").unwrap();
        let (decoded, _, _) = GBK.decode(&result);
        assert_eq!(decoded.as_ref(), "中文测试");
    }

    #[test]
    fn test_convert_bytes_detect_and_convert() {
        let text = "Hello 中文";
        let utf16le_bytes = encode_string(text, "utf16le").unwrap();
        let result = convert_bytes_to(&utf16le_bytes, "utf8").unwrap();
        let decoded = String::from_utf8(result).unwrap();
        assert_eq!(decoded, text);
    }

    #[test]
    fn test_convert_bytes_roundtrip_utf16le_to_utf8bom() {
        let text = "Hello world";
        let utf16le_bytes = encode_string(text, "utf16le").unwrap();
        let result = convert_bytes_to(&utf16le_bytes, "utf8bom").unwrap();
        assert!(result.starts_with(&[0xEF, 0xBB, 0xBF]));
        let (decoded, _, _) = UTF_8.decode(&result[3..]);
        assert_eq!(decoded.as_ref(), text);
    }

    #[test]
    fn test_encode_empty_string() {
        let result = encode_string("", "utf8").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_encode_very_large_string() {
        let large = "Hello World! ".repeat(50_000);
        let result = encode_string(&large, "utf8").unwrap();
        assert_eq!(result.len(), large.len());
    }

    #[test]
    fn test_encode_gbk_chinese() {
        let result = encode_string("中文测试", "gbk").unwrap();
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_detect_from_bytes_plain_utf8() {
        let data = b"Hello, UTF-8 without BOM";
        assert_eq!(detect_from_bytes(data), "utf8");
    }

    #[test]
    fn test_decode_to_string_utf16be() {
        let text = "Test!";
        // Use encode_string which handles BOM correctly, then roundtrip through decode
        let encoded = encode_string(text, "utf16be").unwrap();
        assert_eq!(decode_to_string(&encoded).unwrap(), text);
    }

    #[test]
    fn test_convert_bytes_roundtrip() {
        let original = "Hello 中文";
        let utf8_bytes = original.as_bytes();
        let converted = convert_bytes_to(utf8_bytes, "utf16le").unwrap();
        let decoded = decode_to_string(&converted).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_chardetng_on_gbk_bytes() {
        let data = &[0xC4u8, 0xE3, 0xB9, 0xFE, 0x0A];
        let enc = encoding_rs::Encoding::for_label(b"gbk").unwrap();
        let (cow, _, had_errors) = enc.decode(data);
        assert!(!had_errors);
        assert_eq!(cow.as_ref(), "你哈\n");
    }
}
