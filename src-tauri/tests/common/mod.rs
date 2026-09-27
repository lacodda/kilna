//! Reading the backend's own source, for the gates that have to.
//!
//! Some rules are about code nobody calls in a test - a command whose write
//! skips the log, a journal key with no sentence - so they are read from the
//! source. Until v0.77 each such gate opened the file it was about by path:
//! `commands.rs`, `undo.rs`, four named files for the journal. A file split or
//! moved would have left the gate reading nothing, or reading the wrong half,
//! and still passing. So nothing here takes a path. A gate reads the whole of
//! `src/`, finds what it is about by a marker - an attribute, a call, a
//! function's name - and says so when the marker is not where exactly one of it
//! should be.
//!
//! Test modules are left out of the reading: a `#[cfg(test)]` block builds
//! records and intents of its own, which are fixtures, not what the app does.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri always has a parent")
        .to_path_buf()
}

/// One source file, as the gates read it.
pub struct Source {
    /// Relative to the repository, for messages.
    pub path: String,
    /// The text, with test modules blanked out.
    pub text: String,
    /// The same text with comments and literals blanked too - see [`code_only`].
    pub code: String,
}

/// Every `.rs` under `src-tauri/src`, test modules left out.
pub fn backend() -> Vec<Source> {
    let root = repo_root();
    let mut files = Vec::new();
    walk(&root.join("src-tauri/src"), &mut files);
    files.sort();

    let sources: Vec<Source> = files
        .into_iter()
        .map(|path| {
            let raw = std::fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("{} is readable: {err}", path.display()))
                .replace("\r\n", "\n");
            let text = without_test_modules(&raw);
            let code = code_only(&text);
            Source {
                path: path
                    .strip_prefix(&root)
                    .unwrap_or(&path)
                    .display()
                    .to_string()
                    .replace('\\', "/"),
                text,
                code,
            }
        })
        .collect();

    // The scanner's own watchdog: a walk that found a handful of files is a
    // walk that went somewhere else.
    assert!(
        sources.len() >= 40,
        "read only {} backend source files - the walk has stopped seeing the backend",
        sources.len()
    );
    sources
}

fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("a source directory is readable") {
        let path = entry.expect("a readable entry").path();
        if path.is_dir() {
            walk(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
}

/// The source with every `#[cfg(test)] mod … { … }` blanked to spaces.
///
/// Lengths and line breaks are kept, so an offset still means the same place.
/// Braces are counted in the code-only view, so a `}` inside a string in a
/// test does not end the module early.
pub fn without_test_modules(source: &str) -> String {
    let code = code_only(source);
    let mut out = source.as_bytes().to_vec();
    let mut from = 0;
    while let Some(found) = code[from..].find("#[cfg(test)]") {
        let at = from + found;
        let rest = &code[at + "#[cfg(test)]".len()..];
        let item = rest.trim_start();
        from = at + "#[cfg(test)]".len();
        if !item.starts_with("mod ") {
            continue;
        }
        let Some(open) = code[at..].find('{').map(|offset| at + offset) else {
            continue;
        };
        let close = matching_brace(&code, open);
        for byte in &mut out[at..close] {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
        from = close;
    }
    String::from_utf8(out).expect("blanking keeps the text valid")
}

/// The offset just past the brace that closes the one at `open`.
pub fn matching_brace(code: &str, open: usize) -> usize {
    let mut depth = 0usize;
    for (offset, byte) in code[open..].bytes().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return open + offset + 1;
                }
            }
            _ => {}
        }
    }
    code.len()
}

/// The source with every comment and every string and character literal
/// blanked to spaces, so that what is left is code: a brace inside a
/// `format!` does not open a block, and a word in a comment does not count as
/// a call. Lengths are kept, so an offset means the same place in both.
pub fn code_only(source: &str) -> String {
    blanked(source, true)
}

/// The source with comments blanked and literals kept: for reading the
/// strings a piece of code holds without reading the prose around them.
pub fn without_comments(source: &str) -> String {
    blanked(source, false)
}

fn blanked(source: &str, literals: bool) -> String {
    let bytes = source.as_bytes();
    let mut out = bytes.to_vec();
    let mut i = 0;
    let blank = |out: &mut Vec<u8>, from: usize, to: usize| {
        for byte in &mut out[from..to] {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
    };
    while i < bytes.len() {
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                let end = source[i..].find('\n').map_or(bytes.len(), |at| i + at);
                blank(&mut out, i, end);
                i = end;
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                let end = source[i + 2..]
                    .find("*/")
                    .map_or(bytes.len(), |at| i + 2 + at + 2);
                blank(&mut out, i, end);
                i = end;
            }
            // A raw string: r"..." or r#"..."# with any number of hashes.
            b'r' if matches!(bytes.get(i + 1), Some(b'"') | Some(b'#'))
                && (i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_')) =>
            {
                let hashes = bytes[i + 1..]
                    .iter()
                    .take_while(|byte| **byte == b'#')
                    .count();
                if bytes.get(i + 1 + hashes) != Some(&b'"') {
                    i += 1;
                    continue;
                }
                let close = format!("\"{}", "#".repeat(hashes));
                let start = i + 2 + hashes;
                let end = source[start..]
                    .find(&close)
                    .map_or(bytes.len(), |at| start + at + close.len());
                if literals {
                    blank(&mut out, i, end);
                }
                i = end;
            }
            b'"' => {
                let mut end = i + 1;
                while end < bytes.len() && bytes[end] != b'"' {
                    end += if bytes[end] == b'\\' { 2 } else { 1 };
                }
                if literals {
                    blank(&mut out, i, (end + 1).min(bytes.len()));
                }
                i = end + 1;
            }
            // A character literal ('{', '\n', '\''); a lifetime has no
            // closing quote within the next few bytes and is left alone.
            b'\'' => {
                let end = if bytes.get(i + 1) == Some(&b'\\') {
                    source[i + 2..].find('\'').map(|at| i + 2 + at)
                } else if bytes.get(i + 2) == Some(&b'\'') {
                    Some(i + 2)
                } else {
                    None
                };
                match end {
                    Some(end) if end - i <= 10 => {
                        if literals {
                            blank(&mut out, i, end + 1);
                        }
                        i = end + 1;
                    }
                    _ => i += 1,
                }
            }
            _ => i += 1,
        }
    }
    String::from_utf8(out).expect("blanking keeps the text valid")
}

/// A function found by its name, wherever it lives.
pub struct Function<'a> {
    pub file: &'a Source,
    /// The body, braces included, in the code-only view.
    pub code: &'a str,
    /// The body, braces included, as written (test modules still blanked).
    pub text: &'a str,
}

/// The one function called `name` in the backend.
///
/// Exactly one: none means it was renamed and the gate is reading nothing;
/// two means the gate cannot know which one it is about.
pub fn the_function<'a>(sources: &'a [Source], name: &str) -> Function<'a> {
    let marker = format!("fn {name}(");
    let mut found = Vec::new();
    for file in sources {
        for (at, _) in file.code.match_indices(&marker) {
            // `fn undo(` must not match inside `fn take_undo(`.
            let before = file.code[..at].chars().next_back();
            if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            let Some(open) = file.code[at..].find('{').map(|offset| at + offset) else {
                continue;
            };
            let close = matching_brace(&file.code, open);
            found.push(Function {
                file,
                code: &file.code[open..close],
                text: &file.text[open..close],
            });
        }
    }
    let places: Vec<&str> = found.iter().map(|f| f.file.path.as_str()).collect();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one `fn {name}` in the backend, found it in {places:?}"
    );
    found.remove(0)
}

/// Every string literal that opens a call to `call` - `Record::new("` - across
/// the backend, with where it was found.
pub fn literal_arguments(sources: &[Source], call: &str) -> Vec<(String, String)> {
    let marker = format!("{call}(\"");
    let mut found = Vec::new();
    for file in sources {
        for (at, _) in file.text.match_indices(&marker) {
            // In code, not in a comment or a string that mentions the call.
            if file.code.as_bytes()[at] == b' ' {
                continue;
            }
            let start = at + marker.len();
            let Some(end) = file.text[start..].find('"') else {
                continue;
            };
            found.push((file.path.clone(), file.text[start..start + end].to_owned()));
        }
    }
    found
}

/// Every string literal in a file, as written between its quotes, with the
/// offset of its opening quote. Raw strings included; comments are not read.
pub fn string_literals(file: &Source) -> Vec<(usize, String)> {
    let text = without_comments(&file.text);
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            // A character literal: '"' is a quote, not the start of a string.
            b'\'' if bytes.get(i + 2) == Some(&b'\'') => i += 3,
            b'\'' if bytes.get(i + 1) == Some(&b'\\') && bytes.get(i + 3) == Some(&b'\'') => i += 4,
            b'r' if matches!(bytes.get(i + 1), Some(b'"') | Some(b'#'))
                && (i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_')) =>
            {
                let hashes = bytes[i + 1..].iter().take_while(|b| **b == b'#').count();
                if bytes.get(i + 1 + hashes) != Some(&b'"') {
                    i += 1;
                    continue;
                }
                let close = format!("\"{}", "#".repeat(hashes));
                let start = i + 2 + hashes;
                let end = text[start..]
                    .find(&close)
                    .map_or(bytes.len(), |at| start + at);
                found.push((i, text[start..end].to_owned()));
                i = end + close.len();
            }
            b'"' => {
                let mut end = i + 1;
                while end < bytes.len() && bytes[end] != b'"' {
                    end += if bytes[end] == b'\\' { 2 } else { 1 };
                }
                let end = end.min(bytes.len());
                found.push((i, text[i + 1..end].to_owned()));
                i = end + 1;
            }
            _ => i += 1,
        }
    }
    found
}
