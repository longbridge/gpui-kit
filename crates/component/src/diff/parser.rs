use std::{fmt, sync::Arc};

use gpui::SharedString;

use super::document::{
    DiffDocument, DiffFile, DocumentInner, LinePair, prepare_highlighter, source_lines,
};

/// A malformed or unsupported unified diff, with its one-based patch line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffParseError {
    line: usize,
    message: SharedString,
}

impl DiffParseError {
    pub fn line(&self) -> usize {
        self.line
    }
    pub fn message(&self) -> &SharedString {
        &self.message
    }
    fn new(line: usize, message: &str) -> Self {
        Self {
            line,
            message: message.to_owned().into(),
        }
    }
}
impl fmt::Display for DiffParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "patch line {}: {}", self.line, self.message)
    }
}
impl std::error::Error for DiffParseError {}

type Result<T> = std::result::Result<T, DiffParseError>;

#[derive(Default)]
struct File {
    old_name: Option<String>,
    new_name: Option<String>,
    old: Vec<(usize, String)>,
    new: Vec<(usize, String)>,
    pairs: Vec<LinePair>,
    boundaries: Vec<(usize, SharedString)>,
    metadata: Vec<SharedString>,
    binary: bool,
    additions: usize,
    deletions: usize,
    headers: bool,
    old_no_newline: bool,
    new_no_newline: bool,
}
impl File {
    fn finish(self) -> DiffDocument {
        fn side(
            name: Option<String>,
            fragments: Vec<(usize, String)>,
        ) -> (Option<DiffFile>, Vec<super::document::SourceLine>) {
            let Some(name) = name else {
                return (None, Vec::new());
            };
            let mut source = String::new();
            let mut lines = Vec::with_capacity(fragments.len());
            for (number, fragment) in fragments {
                let offset = source.len();
                // Even an empty unterminated line has a source position.
                let mut line =
                    source_lines(if fragment.is_empty() { "\n" } else { &fragment }).remove(0);
                if fragment.is_empty() {
                    line.source = 0..0;
                    line.content_end = 0;
                }
                line.line_number = number;
                line.source.start += offset;
                line.source.end += offset;
                line.content_end += offset;
                lines.push(line);
                source.push_str(&fragment);
            }
            (
                Some(DiffFile::from_patch(name.into(), source.into())),
                lines,
            )
        }
        let (original, mut original_lines) = side(self.old_name, self.old);
        let (modified, mut modified_lines) = side(self.new_name, self.new);
        for pair in &self.pairs {
            if let (Some(old), Some(new)) = (pair.original, pair.modified) {
                original_lines[old].counterpart = Some(new);
                modified_lines[new].counterpart = Some(old);
            }
        }
        let original_highlighter = prepare_highlighter(original.as_ref());
        let modified_highlighter = prepare_highlighter(modified.as_ref());
        DiffDocument(Arc::new(DocumentInner {
            original,
            modified,
            original_lines,
            modified_lines,
            pairs: self.pairs,
            hunk_boundaries: self.boundaries,
            metadata: self.metadata,
            binary: self.binary,
            additions: self.additions,
            deletions: self.deletions,
            original_highlighter,
            modified_highlighter,
        }))
    }
}

pub(crate) fn parse(patch: SharedString) -> Result<Vec<DiffDocument>> {
    let raw: Vec<&str> = patch
        .split_inclusive('\n')
        .map(|line| line.strip_suffix('\n').unwrap_or(line))
        .collect();
    // Git preserves CR in source lines but emits LF headers. CRLF transport
    // instead has CR on its header records as well; strip that framing CR only.
    let transport_crlf = raw
        .iter()
        .find(|line| line.starts_with("diff --git ") || line.starts_with("--- "))
        .is_some_and(|line| line.ends_with('\r'));
    let lines: Vec<&str> = raw
        .iter()
        .map(|line| {
            if transport_crlf {
                line.strip_suffix('\r').unwrap_or(line)
            } else {
                line
            }
        })
        .collect();
    let mut files = Vec::new();
    let mut current: Option<File> = None;
    let mut ix = 0;
    while ix < lines.len() {
        let line = lines[ix];
        if line.starts_with("diff --cc ")
            || line.starts_with("diff --combined ")
            || line.starts_with("@@@")
        {
            return Err(DiffParseError::new(
                ix + 1,
                "combined diffs are not supported",
            ));
        }
        if let Some(paths) = line.strip_prefix("diff --git ") {
            if let Some(file) = current.take() {
                files.push(file.finish());
            }
            let (old, new) = git_paths(paths)
                .ok_or_else(|| DiffParseError::new(ix + 1, "invalid Git file header"))?;
            current = Some(File {
                old_name: Some(strip_prefix(old)),
                new_name: Some(strip_prefix(new)),
                ..File::default()
            });
            ix += 1;
            continue;
        }
        if let Some(path) = line.strip_prefix("--- ") {
            if current.as_ref().is_some_and(|file| file.headers) {
                files.push(current.take().unwrap().finish());
            }
            let file = current.get_or_insert_with(File::default);
            file.old_name = header_path(path, ix + 1)?;
            ix += 1;
            let Some(path) = lines.get(ix).and_then(|line| line.strip_prefix("+++ ")) else {
                return Err(DiffParseError::new(ix + 1, "expected modified file header"));
            };
            file.new_name = header_path(path, ix + 1)?;
            if file.old_name.is_none() && file.new_name.is_none() {
                return Err(DiffParseError::new(ix + 1, "both file sides are missing"));
            }
            file.headers = true;
            ix += 1;
            continue;
        }
        if line.starts_with("@@ ") {
            let file = current
                .as_mut()
                .ok_or_else(|| DiffParseError::new(ix + 1, "hunk has no file header"))?;
            if !file.headers {
                return Err(DiffParseError::new(
                    ix + 1,
                    "hunk requires original and modified file headers",
                ));
            }
            let (old_start, old_count, new_start, new_count) = hunk_header(line, ix + 1)?;
            if file
                .old
                .last()
                .is_some_and(|(number, _)| *number >= old_start)
                && old_count > 0
                || file
                    .new
                    .last()
                    .is_some_and(|(number, _)| *number >= new_start)
                    && new_count > 0
            {
                return Err(DiffParseError::new(
                    ix + 1,
                    "overlapping or unordered hunks",
                ));
            }
            if file.old_name.is_none() && old_count != 0
                || file.new_name.is_none() && new_count != 0
            {
                return Err(DiffParseError::new(
                    ix + 1,
                    "missing file side has source lines",
                ));
            }
            file.boundaries
                .push((file.pairs.len(), line.to_owned().into()));
            ix += 1;
            let (mut old_used, mut new_used) = (0, 0);
            let (mut deleted, mut added) = (Vec::new(), Vec::new());
            let mut last = None;
            while ix < lines.len() {
                let content = lines[ix];
                if content == "\\ No newline at end of file" {
                    match last {
                        Some(b'-') => {
                            remove_newline(&mut file.old);
                            file.old_no_newline = true;
                        }
                        Some(b'+') => {
                            remove_newline(&mut file.new);
                            file.new_no_newline = true;
                        }
                        Some(b' ') => {
                            remove_newline(&mut file.old);
                            file.old_no_newline = true;
                            remove_newline(&mut file.new);
                            file.new_no_newline = true;
                        }
                        _ => {
                            return Err(DiffParseError::new(
                                ix + 1,
                                "newline marker has no preceding source line",
                            ));
                        }
                    }
                    last = None;
                    ix += 1;
                    continue;
                }
                if old_used == old_count && new_used == new_count {
                    break;
                }
                let Some((&tag, _)) = content.as_bytes().split_first() else {
                    return Err(DiffParseError::new(
                        ix + 1,
                        "hunk line requires a context, addition or deletion prefix",
                    ));
                };
                if !matches!(tag, b' ' | b'+' | b'-') {
                    return Err(DiffParseError::new(
                        ix + 1,
                        "hunk ended before its declared line counts",
                    ));
                }
                if tag != b'+' && file.old_no_newline || tag != b'-' && file.new_no_newline {
                    return Err(DiffParseError::new(
                        ix + 1,
                        "source continues after an unterminated final line",
                    ));
                }
                let source = format!("{}\n", &content[1..]);
                match tag {
                    b' ' => {
                        flush_changes(file, &mut deleted, &mut added);
                        if old_used >= old_count || new_used >= new_count {
                            return Err(DiffParseError::new(
                                ix + 1,
                                "hunk exceeds declared line counts",
                            ));
                        }
                        let old = file.old.len();
                        let new = file.new.len();
                        file.old.push((old_start + old_used, source.clone()));
                        file.new.push((new_start + new_used, source));
                        file.pairs.push(LinePair {
                            original: Some(old),
                            modified: Some(new),
                            changed: false,
                        });
                        old_used += 1;
                        new_used += 1;
                    }
                    b'-' => {
                        if old_used >= old_count {
                            return Err(DiffParseError::new(
                                ix + 1,
                                "hunk exceeds original line count",
                            ));
                        }
                        deleted.push(file.old.len());
                        file.old.push((old_start + old_used, source));
                        old_used += 1;
                        file.deletions += 1;
                    }
                    _ => {
                        if new_used >= new_count {
                            return Err(DiffParseError::new(
                                ix + 1,
                                "hunk exceeds modified line count",
                            ));
                        }
                        added.push(file.new.len());
                        file.new.push((new_start + new_used, source));
                        new_used += 1;
                        file.additions += 1;
                    }
                }
                last = Some(tag);
                ix += 1;
            }
            if old_used != old_count || new_used != new_count {
                return Err(DiffParseError::new(
                    ix + 1,
                    "hunk ended before its declared line counts",
                ));
            }
            flush_changes(file, &mut deleted, &mut added);
            continue;
        }
        // Binary payload is opaque to the UI. Keep its marker, not thousands
        // of encoded records or a fabricated text source.
        if current.as_ref().is_some_and(|file| file.binary) {
            ix += 1;
            continue;
        }
        if line.starts_with('+')
            || line.starts_with('-')
            || line.starts_with(' ')
            || line.starts_with("\\ No newline")
        {
            if current.as_ref().is_some_and(|file| file.headers) {
                return Err(DiffParseError::new(ix + 1, "source line outside a hunk"));
            }
        }
        if let Some(file) = &mut current {
            if line.starts_with("Binary files ") || line == "GIT binary patch" {
                file.binary = true;
            }
            if let Some(path) = line
                .strip_prefix("rename from ")
                .or_else(|| line.strip_prefix("copy from "))
            {
                file.old_name = Some(decode_path(path, ix + 1)?);
            }
            if let Some(path) = line
                .strip_prefix("rename to ")
                .or_else(|| line.strip_prefix("copy to "))
            {
                file.new_name = Some(decode_path(path, ix + 1)?);
            }
            if line.starts_with("new file mode ") {
                file.old_name = None;
            }
            if line.starts_with("deleted file mode ") {
                file.new_name = None;
            }
            if !line.is_empty() {
                file.metadata.push(line.to_owned().into());
            }
        }
        ix += 1;
    }
    if let Some(file) = current {
        files.push(file.finish());
    }
    if files.is_empty() && !patch.trim().is_empty() {
        return Err(DiffParseError::new(1, "expected a unified or Git diff"));
    }
    Ok(files)
}

fn remove_newline(lines: &mut [(usize, String)]) {
    if let Some((_, text)) = lines.last_mut() {
        text.pop();
    }
}
fn flush_changes(file: &mut File, deleted: &mut Vec<usize>, added: &mut Vec<usize>) {
    for ix in 0..deleted.len().max(added.len()) {
        file.pairs.push(LinePair {
            original: deleted.get(ix).copied(),
            modified: added.get(ix).copied(),
            changed: true,
        });
    }
    deleted.clear();
    added.clear();
}
fn hunk_header(line: &str, number: usize) -> Result<(usize, usize, usize, usize)> {
    let invalid = || DiffParseError::new(number, "invalid hunk header");
    let body = line.strip_prefix("@@ ").ok_or_else(invalid)?;
    let (ranges, _) = body.split_once(" @@").ok_or_else(invalid)?;
    let mut fields = ranges.split_whitespace();
    let old = fields
        .next()
        .and_then(|value| value.strip_prefix('-'))
        .ok_or_else(invalid)?;
    let new = fields
        .next()
        .and_then(|value| value.strip_prefix('+'))
        .ok_or_else(invalid)?;
    if fields.next().is_some() {
        return Err(invalid());
    }
    fn range(value: &str) -> Option<(usize, usize)> {
        let (start, count) = value.split_once(',').unwrap_or((value, "1"));
        let start = start.parse::<usize>().ok()?;
        let count = count.parse::<usize>().ok()?;
        if count > 0 && start == 0 || start.checked_add(count).is_none() {
            return None;
        }
        Some((start, count))
    }
    let (old_start, old_count) = range(old).ok_or_else(invalid)?;
    let (new_start, new_count) = range(new).ok_or_else(invalid)?;
    Ok((old_start, old_count, new_start, new_count))
}
fn strip_prefix(path: String) -> String {
    path.strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .unwrap_or(&path)
        .to_owned()
}
fn header_path(value: &str, number: usize) -> Result<Option<String>> {
    let value = value.split('\t').next().unwrap_or(value);
    if value == "/dev/null" {
        Ok(None)
    } else {
        Ok(Some(strip_prefix(decode_path(value, number)?)))
    }
}
fn git_paths(value: &str) -> Option<(String, String)> {
    if value.starts_with('"') {
        let end = quoted_end(value)?;
        let old = decode_path(&value[..end], 1).ok()?;
        let new = decode_path(value[end..].trim_start(), 1).ok()?;
        return Some((old, new));
    }
    // Git does not quote spaces; its b/ delimiter separates file sides.
    let boundary = value.rfind(" b/").or_else(|| value.find(" \"b/"))?;
    Some((
        value[..boundary].to_owned(),
        decode_path(&value[boundary + 1..], 1).ok()?,
    ))
}
fn quoted_end(value: &str) -> Option<usize> {
    let mut escaped = false;
    for (ix, character) in value.char_indices().skip(1) {
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return Some(ix + 1);
        }
    }
    None
}
fn decode_path(value: &str, number: usize) -> Result<String> {
    if !value.starts_with('"') {
        return Ok(value.to_owned());
    }
    if quoted_end(value) != Some(value.len()) {
        return Err(DiffParseError::new(number, "invalid quoted file path"));
    }
    let mut output = Vec::new();
    let bytes = &value.as_bytes()[1..value.len() - 1];
    let mut ix = 0;
    while ix < bytes.len() {
        if bytes[ix] != b'\\' {
            output.push(bytes[ix]);
            ix += 1;
            continue;
        }
        ix += 1;
        let Some(&escaped) = bytes.get(ix) else {
            return Err(DiffParseError::new(number, "invalid path escape"));
        };
        match escaped {
            b'0'..=b'7' => {
                let mut byte = 0u16;
                let mut count = 0;
                while count < 3
                    && bytes
                        .get(ix)
                        .is_some_and(|byte| (b'0'..=b'7').contains(byte))
                {
                    byte = byte * 8 + u16::from(bytes[ix] - b'0');
                    ix += 1;
                    count += 1;
                }
                if byte > 255 {
                    return Err(DiffParseError::new(number, "invalid octal path escape"));
                }
                output.push(byte as u8);
            }
            b'n' => {
                output.push(b'\n');
                ix += 1;
            }
            b't' => {
                output.push(b'\t');
                ix += 1;
            }
            b'r' => {
                output.push(b'\r');
                ix += 1;
            }
            b'a' | b'b' | b'f' | b'v' => {
                output.push(match escaped {
                    b'a' => 7,
                    b'b' => 8,
                    b'f' => 12,
                    _ => 11,
                });
                ix += 1;
            }
            b'"' | b'\\' => {
                output.push(escaped);
                ix += 1;
            }
            _ => return Err(DiffParseError::new(number, "unsupported path escape")),
        }
    }
    String::from_utf8(output)
        .map_err(|_| DiffParseError::new(number, "file path is not valid UTF-8"))
}
