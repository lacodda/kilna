//! A sentence in the backend reads the way it was written.
//!
//! The messages a person sees, the instructions the assistant is given and the
//! reasons the gates give are all string literals, and a long one is split
//! across lines with a trailing backslash, which swallows the indent of the
//! next line. Rewritten by a script that turned that backslash into a real
//! line break and back, the indent stays - as a run of spaces in the middle of
//! the sentence. It compiles, every test passes, and the window says "Install
//! it from                            https://claude.com/claude-code": that is
//! how the message about a missing Claude Code read until v0.77.

mod common;

/// Where a line of a literal has a run of three spaces or more between words.
fn holes(literal: &str) -> Vec<String> {
    let mut found = Vec::new();
    for line in literal.split("\\n").flat_map(|part| part.split('\n')) {
        let trimmed = line.trim();
        let bytes = trimmed.as_bytes();
        let mut run = 0;
        for (index, byte) in bytes.iter().enumerate() {
            if *byte == b' ' {
                run += 1;
                continue;
            }
            // A run of spaces between two words - not an indent, not a
            // column lined up after a colon in a table of values, and not
            // the indent after a `{placeholder}` that ends in a line break.
            if run >= 3 && index > run && !matches!(bytes[index - run - 1], b':' | b'}') {
                found.push(trimmed.to_owned());
                break;
            }
            run = 0;
        }
    }
    found
}

#[test]
fn no_sentence_has_a_hole_of_spaces_in_it() {
    let sources = common::backend();
    let mut read = 0;
    let mut offenders = Vec::new();
    for file in &sources {
        for (at, literal) in common::string_literals(file) {
            read += 1;
            for hole in holes(&literal) {
                let line = file.text[..at].matches('\n').count() + 1;
                offenders.push(format!("{}:{line}: {hole}", file.path));
            }
        }
    }

    // The scanner's watchdog: the backend holds thousands of literals.
    assert!(
        read > 1000,
        "read only {read} string literals - the scan has stopped reading"
    );
    assert!(
        offenders.is_empty(),
        "these literals have a run of spaces inside a line, the mark of a line \
         continuation that lost its backslash:\n  {}",
        offenders.join("\n  ")
    );
}

#[test]
fn the_rule_sees_a_hole_and_nothing_else() {
    assert_eq!(
        holes("Install it from                https://x"),
        ["Install it from                https://x"]
    );
    assert!(holes("one line\\n    indented line").is_empty());
    assert!(holes("key:     value").is_empty());
    assert!(holes("{line}      \"description\"").is_empty());
    assert!(holes("plain sentence, one space each").is_empty());
}
