/// A ZIP archive of `files`, each deflated, dated `date` in ISO 8601 as
/// `2026-09-28T12:00:00Z`.
pub fn zip(files: &[(String, Vec<u8>)], date: &str) -> Vec<u8> {
    let n = |a: usize, b: usize| date.get(a..b).and_then(|s| s.parse().ok()).unwrap_or(0u16);
    let day = ((n(0, 4).max(1980) - 1980) << 9) | (n(5, 7) << 5) | n(8, 10);
    let time = (n(11, 13) << 11) | (n(14, 16) << 5) | (n(17, 19) / 2);
    let (mut out, mut central) = (Vec::new(), Vec::new());
    for (name, data) in files {
        let packed = miniz_oxide::deflate::compress_to_vec(data, 6);
        let head = [
            &[20, 0, 0, 8, 8, 0][..],
            &time.to_le_bytes(),
            &day.to_le_bytes(),
            &crc32fast::hash(data).to_le_bytes(),
            &(packed.len() as u32).to_le_bytes(),
            &(data.len() as u32).to_le_bytes(),
            &(name.len() as u16).to_le_bytes(),
            &[0, 0],
        ]
        .concat();
        central.extend_from_slice(&[0x50, 0x4b, 1, 2, 20, 0]);
        central.extend_from_slice(&head);
        central.extend_from_slice(&[0; 10]);
        central.extend_from_slice(&(out.len() as u32).to_le_bytes());
        central.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&[0x50, 0x4b, 3, 4]);
        out.extend_from_slice(&head);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&packed);
    }
    let (at, size, count) = (out.len() as u32, central.len() as u32, files.len() as u16);
    out.extend_from_slice(&central);
    out.extend_from_slice(&[0x50, 0x4b, 5, 6, 0, 0, 0, 0]);
    out.extend_from_slice(&[count.to_le_bytes(), count.to_le_bytes()].concat());
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&at.to_le_bytes());
    out.extend_from_slice(&[0, 0]);
    out
}

/// The files of `zip` as `zip` writes them, each checked against its CRC.
#[cfg(test)]
pub fn unzip(zip: &[u8]) -> Vec<(String, Vec<u8>)> {
    let u16_at = |i: usize| u16::from_le_bytes([zip[i], zip[i + 1]]) as usize;
    let u32_at = |i: usize| u32::from_le_bytes(zip[i..i + 4].try_into().unwrap());
    let mut files = Vec::new();
    let mut i = 0;
    while zip[i..].starts_with(&[0x50, 0x4b, 3, 4]) {
        let (size, name) = (u32_at(i + 18) as usize, u16_at(i + 26));
        let start = i + 30 + name;
        let data = miniz_oxide::inflate::decompress_to_vec(&zip[start..start + size]).unwrap();
        assert_eq!(crc32fast::hash(&data), u32_at(i + 14));
        assert_eq!(data.len(), u32_at(i + 22) as usize);
        files.push((
            String::from_utf8(zip[i + 30..start].to_vec()).unwrap(),
            data,
        ));
        i = start + size;
    }
    let end = zip.len() - 22;
    assert_eq!(zip[end..end + 4], [0x50, 0x4b, 5, 6]);
    assert_eq!(u16_at(end + 10), files.len());
    assert_eq!(u32_at(end + 16) as usize, i);
    files
}
