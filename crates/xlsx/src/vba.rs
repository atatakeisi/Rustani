//! VBA source import from an Office `vbaProject.bin`, independently implemented from
//! Microsoft's public [MS-OVBA] 2.3.4.2, 2.3.4.3 and 2.4.1 and [MS-CFB].
//!
//! This reads source, not the version-dependent compiled performance cache. It does not
//! execute code, resolve references, remove protection, or rewrite signed VBA projects.
//! Character conversion uses encoding_rs's WHATWG mappings (including Windows-31J for
//! code page 932); obscure Windows code-page mapping differences are not Excel parity.
//! Invalid byte sequences and unknown encodings are reported instead of replaced.
//!
//! Specification: <https://learn.microsoft.com/openspecs/office_file_formats/ms-ovba/>

use crate::cfb::Compound;

const MAX_PROJECT: usize = 32 * 1024 * 1024;
const MAX_DIRECTORY: usize = 1024 * 1024;
const MAX_MODULE: usize = 1024 * 1024;
const MAX_TOTAL_SOURCE: usize = 8 * 1024 * 1024;
const MAX_MODULES: usize = 128;
const MAX_STREAM: usize = 16 * 1024 * 1024;

/// One VBA source module extracted from the project, without executing it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceModule {
    pub name: String,
    pub source: String,
    /// `standard`, `document`, `class`, `form`, or `object` when the PROJECT stream
    /// does not distinguish the object's type. This does not indicate runtime support.
    pub kind: String,
}

struct ModuleInfo {
    name: String,
    stream: String,
    offset: usize,
    standard: bool,
}

/// Extracts bounded, decoded VBA source from a CFB `vbaProject.bin`.
/// Missing source, unsupported encodings, malformed records and exceeded limits are errors;
/// a valid project with no modules returns an empty list. No file system access is required.
pub fn read_modules(project: &[u8]) -> Result<Vec<SourceModule>, String> {
    if project.len() > MAX_PROJECT {
        return Err("VBA project exceeds the 32 MiB import limit".into());
    }
    let compound = Compound::open(project)?;
    let compressed = compound.stream_path(&["VBA", "dir"], MAX_DIRECTORY * 2)?.ok_or("VBA project is missing VBA/dir")?;
    let directory = decompress(&compressed, MAX_DIRECTORY)?;
    let (code_page, entries) = read_directory(&directory)?;
    // Validate the declared code page even for an otherwise empty project.
    let _ = decode(&[], code_page)?;
    let manifest = compound.stream_path(&["PROJECT"], MAX_DIRECTORY)?.map(|b| decode(&b, code_page)).transpose()?;
    let mut modules = Vec::with_capacity(entries.len());
    let mut total = 0usize;
    let mut stream_budget = MAX_PROJECT;
    for info in entries {
        let stream = compound
            .stream_path(&["VBA", &info.stream], MAX_STREAM.min(stream_budget))?
            .ok_or_else(|| format!("VBA module {} is missing its source stream", info.name))?;
        stream_budget = stream_budget.saturating_sub(stream.len());
        let bytes = stream.get(info.offset..).ok_or_else(|| format!("VBA module {} has an out-of-range source offset", info.name))?;
        let remaining = MAX_TOTAL_SOURCE.saturating_sub(total);
        let source = decode(&decompress(bytes, MAX_MODULE.min(remaining))?, code_page)?;
        if source.len() > MAX_MODULE || source.len() > remaining {
            return Err("decoded VBA source exceeds the 1 MiB module or 8 MiB project limit".into());
        }
        if source.contains('\0') {
            return Err(format!("VBA module {} contains a null character", info.name));
        }
        total = total.saturating_add(source.len());
        let kind = module_kind(manifest.as_deref(), &info);
        modules.push(SourceModule { name: info.name, source, kind: kind.to_string() });
    }
    Ok(modules)
}

fn module_kind(manifest: Option<&str>, info: &ModuleInfo) -> &'static str {
    if info.standard {
        return "standard";
    }
    for line in manifest.unwrap_or_default().lines() {
        let Some((key, value)) = line.split_once('=') else { continue };
        let value = value.split('/').next().unwrap_or_default();
        if value.eq_ignore_ascii_case(&info.name) {
            match key {
                "Document" => return "document",
                "Class" => return "class",
                "BaseClass" => return "form",
                _ => {}
            }
        }
    }
    "object"
}

/// MS-OVBA compressed containers: chunks reset their copy dictionary every 4096 bytes.
/// Copying byte-by-byte intentionally supports overlapping backreferences.
fn decompress(bytes: &[u8], limit: usize) -> Result<Vec<u8>, String> {
    if bytes.first() != Some(&1) {
        return Err("invalid VBA compressed container signature".into());
    }
    let mut cursor = Cursor::new(bytes.get(1..).ok_or("truncated VBA compressed container")?);
    let mut out = Vec::new();
    while !cursor.is_empty() {
        let header = cursor.u16()?;
        if header & 0x7000 != 0x3000 {
            return Err("invalid VBA compressed chunk signature".into());
        }
        let size = usize::from(header & 0x0FFF) + 1;
        let chunk = cursor.take(size)?;
        let start = out.len();
        if header & 0x8000 == 0 {
            if size != 4096 {
                return Err("a raw VBA chunk must contain 4096 bytes".into());
            }
            if size > limit.saturating_sub(out.len()) {
                return Err("VBA decompression limit exceeded".into());
            }
            out.extend_from_slice(chunk);
            continue;
        }
        let mut tokens = Cursor::new(chunk);
        while !tokens.is_empty() {
            let flags = tokens.byte()?;
            for bit in 0..8 {
                if tokens.is_empty() {
                    break;
                }
                let produced = out.len().saturating_sub(start);
                if flags & (1 << bit) == 0 {
                    if produced >= 4096 || out.len() >= limit {
                        return Err("VBA decompression limit exceeded".into());
                    }
                    out.push(tokens.byte()?);
                } else {
                    let token = tokens.u16()?;
                    if produced == 0 {
                        return Err("VBA copy token precedes its dictionary".into());
                    }
                    let offset_bits = (usize::BITS - produced.saturating_sub(1).leading_zeros()).max(4);
                    let length_bits = 16u32.saturating_sub(offset_bits);
                    let mask = 0xFFFFu16 >> offset_bits;
                    let count = usize::from(token & mask) + 3;
                    let distance = usize::from(token >> length_bits) + 1;
                    if distance > produced || count > 4096usize.saturating_sub(produced) {
                        return Err("invalid VBA copy token bounds".into());
                    }
                    if count > limit.saturating_sub(out.len()) {
                        return Err("VBA decompression limit exceeded".into());
                    }
                    for _ in 0..count {
                        let at = out.len().checked_sub(distance).ok_or("invalid VBA copy token offset")?;
                        let byte = out.get(at).copied().ok_or("invalid VBA copy token dictionary")?;
                        out.push(byte);
                    }
                }
            }
        }
    }
    Ok(out)
}

/// Reads fields in their specified order; never guesses boundaries by scanning for ids.
fn read_directory(bytes: &[u8]) -> Result<(u16, Vec<ModuleInfo>), String> {
    let mut c = Cursor::new(bytes);
    c.fixed(0x0001, 4)?; // PROJECTSYSKIND
    if c.peek_u16()? == 0x004A {
        c.fixed(0x004A, 4)?; // PROJECTCOMPATVERSION
    }
    c.fixed(0x0002, 4)?; // PROJECTLCID
    c.fixed(0x0014, 4)?; // PROJECTLCIDINVOKE
    let code_page = Cursor::new(c.fixed(0x0003, 2)?).u16()?;
    c.record(0x0004)?; // PROJECTNAME
    c.pair(0x0005, 0x0040)?; // PROJECTDOCSTRING / Unicode
    c.pair(0x0006, 0x003D)?; // PROJECTHELPFILEPATH
    c.fixed(0x0007, 4)?;
    c.fixed(0x0008, 4)?;
    c.id(0x0009)?; // PROJECTVERSION is not a length-prefixed record.
    c.u32()?; // Reserved: ignored on read.
    c.take(6)?;
    if c.peek_u16()? == 0x000C {
        c.pair(0x000C, 0x003C)?; // PROJECTCONSTANTS
    }
    let mut references = 0usize;
    while c.peek_u16()? != 0x000F {
        references += 1;
        if references > 1024 {
            return Err("VBA project exceeds the reference record limit".into());
        }
        if c.peek_u16()? == 0x0016 {
            c.pair(0x0016, 0x003E)?;
        }
        match c.peek_u16()? {
            0x000D => {
                c.id(0x000D)?;
                c.u32()?; // Total Size: ignored on read.
                c.sized()?;
                c.take(6)?;
            }
            0x000E => {
                c.id(0x000E)?;
                c.u32()?;
                c.sized()?;
                c.sized()?;
                c.take(6)?;
            }
            0x0033 => {
                c.record(0x0033)?;
                read_control_reference(&mut c)?;
            }
            0x002F => read_control_reference(&mut c)?,
            id => return Err(format!("unsupported VBA reference record 0x{id:04X}")),
        }
    }
    let count = usize::from(Cursor::new(c.fixed(0x000F, 2)?).u16()?);
    if count > MAX_MODULES {
        return Err("VBA project exceeds the 128-module limit".into());
    }
    c.fixed(0x0013, 2)?; // PROJECTCOOKIE
    let mut modules: Vec<ModuleInfo> = Vec::with_capacity(count);
    for _ in 0..count {
        let name_bytes = c.record(0x0019)?;
        let mut name = decode(name_bytes, code_page)?;
        if c.peek_u16()? == 0x0047 {
            name = unicode(c.record(0x0047)?)?;
        }
        let stream_bytes = c.record(0x001A)?;
        let ansi_stream = decode(stream_bytes, code_page)?;
        let stream = unicode(c.record(0x0032)?)?;
        if stream.is_empty() || ansi_stream.is_empty() {
            return Err("VBA module has an empty stream name".into());
        }
        validate_name(&name)?;
        validate_name(&stream)?;
        if modules.iter().any(|m| m.name.eq_ignore_ascii_case(&name) || m.stream.eq_ignore_ascii_case(&stream)) {
            return Err("VBA project contains duplicate module names or streams".into());
        }
        c.pair(0x001C, 0x0048)?; // MODULEDOCSTRING
        let offset = usize::try_from(Cursor::new(c.fixed(0x0031, 4)?).u32()?).map_err(|_| "VBA source offset exceeds address space")?;
        c.fixed(0x001E, 4)?;
        c.fixed(0x002C, 2)?;
        let standard = match c.u16()? {
            0x0021 => true,
            0x0022 => false,
            id => return Err(format!("unsupported VBA module type 0x{id:04X}")),
        };
        c.u32()?; // MODULETYPE Reserved
        if c.peek_u16()? == 0x0025 {
            c.id(0x0025)?;
            c.u32()?; // MODULEREADONLY Reserved
        }
        if c.peek_u16()? == 0x0028 {
            c.id(0x0028)?;
            c.u32()?; // MODULEPRIVATE Reserved
        }
        c.id(0x002B)?;
        c.u32()?; // Module terminator's Reserved, ignored on read.
        modules.push(ModuleInfo { name, stream, offset, standard });
    }
    c.id(0x0010)?;
    c.u32()?; // Directory terminator's Reserved, ignored on read.
    if !c.is_empty() {
        return Err("VBA directory has trailing unsupported records".into());
    }
    Ok((code_page, modules))
}

fn read_control_reference(c: &mut Cursor<'_>) -> Result<(), String> {
    c.id(0x002F)?;
    c.u32()?; // SizeTwiddled
    c.sized()?;
    c.take(6)?;
    if c.peek_u16()? == 0x0016 {
        c.pair(0x0016, 0x003E)?;
    }
    c.id(0x0030)?;
    c.u32()?; // SizeExtended
    c.sized()?;
    c.take(26)?; // Reserved4, Reserved5, OriginalTypeLib, Cookie
    Ok(())
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.chars().count() > 128 || name.chars().any(char::is_control) {
        return Err("VBA module name is empty, too long or contains a control character".into());
    }
    Ok(())
}

fn unicode(bytes: &[u8]) -> Result<String, String> {
    if !bytes.len().is_multiple_of(2) {
        return Err("VBA Unicode field has an odd length".into());
    }
    let units: Vec<u16> = bytes.as_chunks::<2>().0.iter().copied().map(u16::from_le_bytes).collect();
    String::from_utf16(&units).map_err(|_| "VBA Unicode field contains an invalid surrogate".into())
}

fn decode(bytes: &[u8], code_page: u16) -> Result<String, String> {
    if code_page == 20127 {
        return if bytes.is_ascii() {
            String::from_utf8(bytes.to_vec()).map_err(|_| "invalid ASCII VBA source".into())
        } else {
            Err("VBA source contains non-ASCII bytes in code page 20127".into())
        };
    }
    let label = match code_page {
        65001 => "utf-8".to_string(),
        932 => "shift_jis".to_string(),
        936 => "gbk".to_string(),
        949 => "euc-kr".to_string(),
        950 => "big5".to_string(),
        874 => "windows-874".to_string(),
        1250..=1258 => format!("windows-{code_page}"),
        866 => "ibm866".to_string(),
        10000 => "macintosh".to_string(),
        10007 => "x-mac-cyrillic".to_string(),
        20866 => "koi8-r".to_string(),
        21866 => "koi8-u".to_string(),
        _ => return Err(format!("unsupported VBA project code page {code_page}")),
    };
    let encoding = encoding_rs::Encoding::for_label(label.as_bytes()).ok_or_else(|| format!("unsupported VBA project code page {code_page}"))?;
    encoding
        .decode_without_bom_handling_and_without_replacement(bytes)
        .map(|s| s.into_owned())
        .ok_or_else(|| format!("invalid byte sequence for VBA project code page {code_page}"))
}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn is_empty(&self) -> bool {
        self.at == self.bytes.len()
    }

    fn take(&mut self, size: usize) -> Result<&'a [u8], String> {
        let end = self.at.checked_add(size).ok_or("VBA field length overflows address space")?;
        let value = self.bytes.get(self.at..end).ok_or("truncated VBA record or compressed chunk")?;
        self.at = end;
        Ok(value)
    }

    fn byte(&mut self) -> Result<u8, String> {
        self.take(1)?.first().copied().ok_or_else(|| "truncated VBA byte".into())
    }

    fn peek_u16(&self) -> Result<u16, String> {
        let end = self.at.checked_add(2).ok_or("VBA cursor overflow")?;
        let value = self.bytes.get(self.at..end).ok_or("truncated VBA record identifier")?;
        let array = <[u8; 2]>::try_from(value).map_err(|_| "truncated VBA record identifier")?;
        Ok(u16::from_le_bytes(array))
    }

    fn u16(&mut self) -> Result<u16, String> {
        let array = <[u8; 2]>::try_from(self.take(2)?).map_err(|_| "truncated VBA integer")?;
        Ok(u16::from_le_bytes(array))
    }

    fn u32(&mut self) -> Result<u32, String> {
        let array = <[u8; 4]>::try_from(self.take(4)?).map_err(|_| "truncated VBA integer")?;
        Ok(u32::from_le_bytes(array))
    }

    fn id(&mut self, expected: u16) -> Result<(), String> {
        let id = self.u16()?;
        if id != expected {
            return Err(format!("unsupported or malformed VBA record: expected 0x{expected:04X}, found 0x{id:04X}"));
        }
        Ok(())
    }

    fn sized(&mut self) -> Result<&'a [u8], String> {
        let size = usize::try_from(self.u32()?).map_err(|_| "VBA field length exceeds address space")?;
        self.take(size)
    }

    fn record(&mut self, id: u16) -> Result<&'a [u8], String> {
        self.id(id)?;
        self.sized()
    }

    fn fixed(&mut self, id: u16, size: usize) -> Result<&'a [u8], String> {
        let value = self.record(id)?;
        if value.len() != size {
            return Err(format!("invalid length of VBA record 0x{id:04X}"));
        }
        Ok(value)
    }

    fn pair(&mut self, id: u16, second: u16) -> Result<(), String> {
        self.record(id)?;
        self.record(second)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn record(bytes: &mut Vec<u8>, id: u16, data: &[u8]) {
        bytes.extend(id.to_le_bytes());
        bytes.extend((data.len() as u32).to_le_bytes());
        bytes.extend(data);
    }

    fn wide(value: &str) -> Vec<u8> {
        value.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

    fn directory(count: u16, code_page: u16, offset: u32, standard: bool) -> Vec<u8> {
        let mut bytes = Vec::new();
        record(&mut bytes, 1, &1u32.to_le_bytes());
        record(&mut bytes, 2, &0x409u32.to_le_bytes());
        record(&mut bytes, 0x14, &0x409u32.to_le_bytes());
        record(&mut bytes, 3, &code_page.to_le_bytes());
        record(&mut bytes, 4, b"SyntheticProject");
        for (first, second) in [(5, 0x40), (6, 0x3D)] {
            record(&mut bytes, first, &[]);
            record(&mut bytes, second, &[]);
        }
        record(&mut bytes, 7, &0u32.to_le_bytes());
        record(&mut bytes, 8, &0u32.to_le_bytes());
        record(&mut bytes, 9, &1u32.to_le_bytes());
        bytes.extend(0u16.to_le_bytes()); // MinorVersion: outside the reserved "size".
        record(&mut bytes, 0xF, &count.to_le_bytes());
        record(&mut bytes, 0x13, &0xFFFFu16.to_le_bytes());
        for index in 0..count.min(129) {
            let name = if index == 0 { "Example".to_string() } else { format!("Example{index}") };
            let stream = if index == 0 { "CodeStream".to_string() } else { format!("CodeStream{index}") };
            record(&mut bytes, 0x19, name.as_bytes());
            record(&mut bytes, 0x47, &wide(&name));
            record(&mut bytes, 0x1A, stream.as_bytes());
            record(&mut bytes, 0x32, &wide(&stream));
            record(&mut bytes, 0x1C, &[]);
            record(&mut bytes, 0x48, &[]);
            record(&mut bytes, 0x31, &offset.to_le_bytes());
            record(&mut bytes, 0x1E, &0u32.to_le_bytes());
            record(&mut bytes, 0x2C, &0xFFFFu16.to_le_bytes());
            record(&mut bytes, if standard { 0x21 } else { 0x22 }, &[]);
            record(&mut bytes, 0x2B, &[]);
        }
        record(&mut bytes, 0x10, &[]);
        bytes
    }

    fn literal_container(bytes: &[u8]) -> Vec<u8> {
        assert!(bytes.len() <= 3600);
        let mut out = vec![1];
        if bytes.is_empty() {
            return out;
        }
        let mut encoded = Vec::new();
        for group in bytes.chunks(8) {
            encoded.push(0); // Eight literal tokens, or the remaining last tokens.
            encoded.extend(group);
        }
        out.extend((0xB000u16 | (encoded.len() as u16 - 1)).to_le_bytes());
        out.extend(encoded);
        out
    }

    /// Start with the original test CFB writer, then make entry 1 a VBA storage with
    /// dir and CodeStream children. The PROJECT manifest remains under the root.
    fn project(dir: &[u8], module: &[u8], manifest: &[u8]) -> Vec<u8> {
        project_with_modules(dir, &[("CodeStream", module)], manifest)
    }

    fn project_with_modules(dir: &[u8], modules: &[(&str, &[u8])], manifest: &[u8]) -> Vec<u8> {
        let compressed_dir = literal_container(dir);
        let mut streams = vec![("VBA", &[][..]), ("dir", &compressed_dir[..])];
        streams.extend_from_slice(modules);
        streams.push(("PROJECT", manifest));
        let mut file = crate::testing::compound_file(&streams);
        let start = u32::from_le_bytes(file[48..52].try_into().unwrap()) as usize;
        let at = (start + 1) * 512;
        let put32 = |data: &mut [u8], index: usize, value: u32| data[index..index + 4].copy_from_slice(&value.to_le_bytes());
        file[at + 128 + 66] = 1; // Storage, not a stream.
        put32(&mut file, at + 128 + 72, (3 + modules.len()) as u32); // VBA's sibling is PROJECT.
        put32(&mut file, at + 128 + 76, 2); // VBA owns dir.
        put32(&mut file, at + (2 + modules.len()) * 128 + 72, 0xFFFF_FFFF); // Last child.
        file
    }

    #[test]
    fn imports_actual_nested_streams_and_ignores_performance_cache() {
        let source = b"Attribute VB_Name = \"Example\"\r\nSub Run()\r\nRange(\"A1\").Value = 42\r\nEnd Sub\r\n";
        let mut stream = b"uninterpreted cache".to_vec();
        let offset = stream.len() as u32;
        stream.extend(literal_container(source));
        let file = project(&directory(1, 1252, offset, true), &stream, b"Module=Example\r\n");
        let modules = read_modules(&file).unwrap();
        assert_eq!(
            modules,
            vec![SourceModule { name: "Example".into(), source: String::from_utf8(source.to_vec()).unwrap(), kind: "standard".into() }]
        );
    }

    #[test]
    fn imports_japanese_source_without_replacement() {
        let source = "Sub Run()\r\nRange(\"A1\").Value = \"日本語\"\r\nEnd Sub\r\n";
        let (encoded, _, had_errors) = encoding_rs::SHIFT_JIS.encode(source);
        assert!(!had_errors);
        let file = project(&directory(1, 932, 0, false), &literal_container(&encoded), b"Document=Example/&H00000000\r\n");
        let modules = read_modules(&file).unwrap();
        assert_eq!(modules[0].source, source);
        assert_eq!(modules[0].kind, "document");
        assert!(decode(&[0x82], 932).is_err());
        assert!(decode(b"text", 42).is_err());
        assert!(decode(&[0xFF], 65001).is_err());
    }

    #[test]
    fn compressed_backreferences_overlap_and_reset_per_chunk() {
        // A literal 'a', then an offset-one copy token producing six more 'a's.
        let compressed = [1, 3, 0xB0, 2, b'a', 3, 0];
        assert_eq!(decompress(&compressed, 7).unwrap(), b"aaaaaaa");
        assert!(decompress(&compressed, 6).is_err());
        let mut raw = vec![1, 0xFF, 0x3F];
        raw.extend(vec![b'x'; 4096]);
        raw.extend([2, 0xB0, 1, 0, 0]); // New chunk starts with a copy: invalid.
        assert!(decompress(&raw, 8192).is_err());
    }

    #[test]
    fn copy_tokens_change_bit_width_at_the_sixteen_byte_boundary() {
        for prefix_size in [16usize, 17, 32, 33, 128, 129] {
            let prefix: Vec<u8> = (0..prefix_size).map(|n| b'A' + (n % 26) as u8).collect();
            let mut payload = Vec::new();
            let mut tokens: Vec<(bool, Vec<u8>)> = prefix.iter().map(|b| (false, vec![*b])).collect();
            let offset_bits = (usize::BITS - (prefix_size - 1).leading_zeros()).max(4);
            let token = ((prefix_size - 1) as u16) << (16 - offset_bits); // distance=prefix size, length=3
            tokens.push((true, token.to_le_bytes().to_vec()));
            for group in tokens.chunks(8) {
                let flags = group.iter().enumerate().fold(0u8, |flags, (bit, (copy, _))| flags | (u8::from(*copy) << bit));
                payload.push(flags);
                for (_, token) in group {
                    payload.extend(token);
                }
            }
            let mut compressed = vec![1];
            compressed.extend((0xB000u16 | (payload.len() as u16 - 1)).to_le_bytes());
            compressed.extend(payload);
            let mut expected = prefix.clone();
            expected.extend_from_slice(&prefix[..3]);
            assert_eq!(decompress(&compressed, expected.len()).unwrap(), expected);
        }
    }

    #[test]
    fn references_are_parsed_structurally_before_modules() {
        let mut dir = directory(1, 1252, 0, true);
        let marker = [0x0F, 0, 2, 0, 0, 0, 1, 0];
        let modules_at = dir.windows(marker.len()).position(|v| v == marker).unwrap();
        let mut references = Vec::new();
        record(&mut references, 0x16, b"SyntheticLibrary");
        record(&mut references, 0x3E, &wide("SyntheticLibrary"));
        references.extend(0xDu16.to_le_bytes());
        references.extend(0u32.to_le_bytes()); // Aggregate size is ignored by the format.
        references.extend(3u32.to_le_bytes());
        references.extend(b"abc");
        references.extend([0; 6]);
        record(&mut references, 0x33, b"OriginalLibrary");
        references.extend(0x2Fu16.to_le_bytes());
        references.extend(0u32.to_le_bytes());
        references.extend(2u32.to_le_bytes());
        references.extend(b"tw");
        references.extend([0; 6]);
        references.extend(0x30u16.to_le_bytes());
        references.extend(0u32.to_le_bytes());
        references.extend(2u32.to_le_bytes());
        references.extend(b"ex");
        references.extend([0; 26]);
        dir.splice(modules_at..modules_at, references);
        let (_, modules) = read_directory(&dir).unwrap();
        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].stream, "CodeStream");
    }

    #[test]
    fn raw_chunks_and_output_bombs_are_bounded() {
        let mut raw = vec![1, 0xFF, 0x3F];
        raw.extend(vec![b'x'; 4096]);
        assert_eq!(decompress(&raw, 4096).unwrap(), vec![b'x'; 4096]);
        assert!(decompress(&raw, 4095).is_err());
        // One literal followed by the largest copy token fills a 4096-byte chunk.
        let mut bomb = vec![1];
        for _ in 0..MAX_MODULE / 4096 + 1 {
            bomb.extend([3, 0xB0, 2, b'x', 0xFC, 0x0F]);
        }
        assert!(decompress(&bomb, MAX_MODULE).is_err());
        for bad in [&[][..], &[0], &[1, 0], &[1, 0, 0], &[1, 0, 0x30, 0], &[1, 2, 0xB0, 1, 0, 0]] {
            assert!(decompress(bad, MAX_MODULE).is_err());
        }
    }

    #[test]
    fn combined_source_budget_and_large_module_sources_are_bounded() {
        let mut compressed = vec![1];
        for _ in 0..MAX_MODULE / 4096 {
            compressed.extend([3, 0xB0, 2, b'x', 0xFC, 0x0F]);
        }
        let names: Vec<String> = (0..9).map(|index| if index == 0 { "CodeStream".into() } else { format!("CodeStream{index}") }).collect();
        let modules: Vec<(&str, &[u8])> = names.iter().map(|name| (name.as_str(), compressed.as_slice())).collect();
        let file = project_with_modules(&directory(8, 1252, 0, true), &modules[..8], b"");
        assert_eq!(read_modules(&file).unwrap().iter().map(|module| module.source.len()).sum::<usize>(), MAX_TOTAL_SOURCE);
        let file = project_with_modules(&directory(9, 1252, 0, true), &modules, b"");
        assert!(read_modules(&file).is_err());
        compressed.extend([3, 0xB0, 2, b'x', 0xFC, 0x0F]);
        assert!(read_modules(&project(&directory(1, 1252, 0, true), &compressed, b"")).is_err());
        assert_eq!(read_directory(&directory(128, 1252, 0, true)).unwrap().1.len(), 128);
    }

    #[test]
    fn truncated_records_offsets_unknown_records_and_module_limits_error() {
        let dir = directory(1, 1252, 0, true);
        for cut in 0..dir.len() {
            assert!(read_directory(&dir[..cut]).is_err(), "cut {cut}");
        }
        assert!(read_directory(&directory(129, 1252, 0, true)).is_err());
        assert!(read_directory(&directory(0, 1252, 0, true)).unwrap().1.is_empty());
        let file = project(&directory(1, 1252, 9000, true), &literal_container(b"Sub Run()\r\nEnd Sub"), b"");
        assert!(read_modules(&file).is_err());
        let mut invalid = dir.clone();
        invalid[0..2].copy_from_slice(&0xFFFFu16.to_le_bytes());
        assert!(read_directory(&invalid).is_err());
        for cut in [0, 8, 100, 600, file.len() - 1] {
            assert!(read_modules(&file[..cut]).is_err());
        }
    }

    #[test]
    fn cyclic_directory_and_missing_source_are_errors() {
        let mut file = project(&directory(1, 1252, 0, true), &literal_container(b"Sub Run()\r\nEnd Sub"), b"");
        let start = u32::from_le_bytes(file[48..52].try_into().unwrap()) as usize;
        let dir_at = (start + 1) * 512;
        file[dir_at + 2 * 128 + 72..dir_at + 2 * 128 + 76].copy_from_slice(&2u32.to_le_bytes());
        assert!(read_modules(&file).is_err());
        assert!(read_modules(&crate::testing::compound_file(&[("PROJECT", b"")])).is_err());
    }

    #[test]
    fn malformed_compression_mutations_never_panic() {
        let original = literal_container(b"Sub Run()\r\nEnd Sub\r\n");
        for at in 0..original.len() {
            for value in [0, 1, 0x7F, 0x80, 0xFF] {
                let mut bytes = original.clone();
                bytes[at] = value;
                let _ = decompress(&bytes, MAX_MODULE);
            }
        }
    }
}
