//! Map byte offsets in a **stitched** source buffer to logical file paths (`# sym:file` markers).

const MARKER: &str = "# sym:file ";

/// `(content_start_byte, content_end_byte_exclusive, path)`
fn stitched_segments(stitched: &str) -> Vec<(usize, usize, String)> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < stitched.len() {
        let line_start = i;
        let rel = &stitched[line_start..];
        if rel.starts_with(MARKER) {
            let after = line_start + MARKER.len();
            let rest = &stitched[after..];
            let Some(nl) = rest.find('\n') else {
                break;
            };
            let path = rest[..nl].trim().to_string();
            let content_start = after + nl + 1;
            let content_end = find_next_marker_or_end(stitched, content_start);
            out.push((content_start, content_end, path));
            if content_end >= stitched.len() {
                break;
            }
            i = content_end + 1;
            continue;
        }
        if let Some(p) = stitched[i..].find('\n') {
            i = i + p + 1;
        } else {
            break;
        }
    }
    out
}

/// Exclusive end of segment content: byte index of `\n` that precedes the next `\n# sym:file `, or `len`.
fn find_next_marker_or_end(stitched: &str, from: usize) -> usize {
    stitched[from..]
        .find("\n# sym:file ")
        .map(|p| from + p)
        .unwrap_or(stitched.len())
}

fn line_col_in_slice(slice: &str, byte_in_slice: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, c) in slice.char_indices() {
        if i >= byte_in_slice {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// Path + 1-based line/column **within this file’s stitched segment body** (not global stitched buffer).
pub fn logical_position(stitched: &str, byte_offset: usize) -> Option<(String, usize, usize)> {
    if !stitched.contains(MARKER) {
        return None;
    }
    for (start, end, path) in stitched_segments(stitched) {
        if byte_offset < start || byte_offset > end {
            continue;
        }
        let slice = stitched.get(start..end)?;
        let max_rel = slice.len().saturating_sub(1);
        let rel = (byte_offset - start).min(max_rel);
        let (line, col) = line_col_in_slice(slice, rel);
        return Some((path, line, col));
    }
    None
}

/// If `stitched` contains `# sym:file` markers, return the logical path for `byte_offset`.
pub fn logical_file_for_byte(stitched: &str, byte_offset: usize) -> Option<String> {
    logical_position(stitched, byte_offset).map(|(p, _, _)| p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_segments_second_file() {
        let s = concat!(
            "# sym:file /proj/a.sym\n",
            "fn main() -> Int = 1 end\n\n",
            "# sym:file /proj/b.sym\n",
            "fn bad() -> Int = true end\n\n",
        );
        let err_off = s.find("true").expect("true");
        assert_eq!(
            logical_file_for_byte(s, err_off).as_deref(),
            Some("/proj/b.sym")
        );
        let (p, line, col) = logical_position(s, err_off).expect("loc");
        assert_eq!(p, "/proj/b.sym");
        assert_eq!(line, 1);
        assert!(col > 1, "column should point into line");
    }

    #[test]
    fn second_line_in_segment() {
        let s = concat!(
            "# sym:file /p/x.sym\n",
            "fn a() -> Int = 1 end\n",
            "fn b() -> Int = true end\n",
        );
        let off = s.find("true").expect("true");
        let (_, line, _) = logical_position(s, off).expect("loc");
        assert_eq!(line, 2);
    }
}
