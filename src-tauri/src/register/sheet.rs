//! The register and the neighbours as the assistant reads them: `{register}`
//! and `{neighbours}` in a template.
//!
//! Generated every time from the rows, so what a prompt is told can never
//! drift from what the screen shows - the reason the predecessor's digest of
//! banned words, kept by hand in a file, was retired.

use rusqlite::Connection;

use super::neighbours::Neighbour;
use super::{RegisterEntry, Strictness};
use crate::error::Result;

/// How many works `{neighbours}` hands over, and how much of each.
pub const NEIGHBOURS: usize = 6;
const NEIGHBOUR_TEXT: usize = 1800;

/// The register for `{register}`: wording by strictness, then the meanings,
/// each with how many works carry it and why it is spent - and, when there is
/// a text, what that text already takes.
pub fn register(conn: &Connection, profile_id: &str, text: Option<&str>) -> Result<String> {
    // The register is what is spent: a word kept for a song to come, or a
    // way of singing, is no term of it (ADR 0052).
    let entries: Vec<_> = super::entries(conn, profile_id)?
        .into_iter()
        .filter(|e| e.term.strictness.is_some())
        .collect();
    if entries.is_empty() {
        return Ok("(the register of repeats is empty)".to_owned());
    }
    let mut out = String::from(
        "The register of repeats: what this body of work has already spent. The number is \
         how many works carry each term now.\n",
    );
    let wording = |strictness: Strictness| {
        entries
            .iter()
            .filter(move |e| e.term.kind.is_wording() && e.term.strictness == Some(strictness))
    };
    for (strictness, heading) in [
        (Strictness::Ban, "Banned - do not use"),
        (
            Strictness::Limit,
            "Limited - each use has to earn its place",
        ),
        (
            Strictness::Rare,
            "Rare but conspicuous - even two or three uses stand out",
        ),
    ] {
        let lines: Vec<String> = wording(strictness).map(line).collect();
        if !lines.is_empty() {
            out.push_str(&format!("\n{heading}:\n{}\n", lines.join("\n")));
        }
    }
    let meanings: Vec<String> = entries
        .iter()
        .filter(|e| !e.term.kind.is_wording())
        .map(|e| {
            format!(
                "{} [{}]",
                line(e),
                e.term.strictness.unwrap_or_default().as_str()
            )
        })
        .collect();
    if !meanings.is_empty() {
        out.push_str(&format!(
            "\nSpent images, scenes and devices - told in any words, they still count:\n{}\n",
            meanings.join("\n")
        ));
    }

    if let Some(text) = text {
        let checked = super::check::text(conn, profile_id, text, false)?;
        let taken: Vec<String> = checked
            .terms
            .iter()
            .map(|hit| format!("{} ×{} ({})", hit.word, hit.count, hit.strictness.as_str()))
            .collect();
        out.push_str(&if taken.is_empty() {
            "\nThis text takes no term of the register word for word.\n".to_owned()
        } else {
            format!(
                "\nThis text already takes, word for word: {}.\n",
                taken.join(", ")
            )
        });
    }
    Ok(out.trim_end().to_owned())
}

fn line(entry: &RegisterEntry) -> String {
    let term = &entry.term;
    let mut line = format!("- {}", term.word);
    if !term.forms.is_empty() {
        line.push_str(&format!(" / {}", term.forms.join(" / ")));
    }
    line.push_str(&format!(" ({})", entry.uses));
    if let Some(note) = term.note.as_deref() {
        line.push_str(&format!(" - {}", note.replace('\n', " ")));
    }
    line
}

/// The words for `{words}` and `{words:<block>}`: the fresh words of the
/// owner's bank - all of them by block, or one block's - each with how it is
/// sung where that is not how it is written (ADR 0052). A block nobody has
/// is said so rather than read as an empty bank: a template that names a
/// block renamed since is a mistake worth seeing in the prompt.
pub fn words(conn: &Connection, profile_id: &str, block: Option<&str>) -> Result<String> {
    use super::{Bank, Term, block as blocks};
    let said = |term: &Term| -> String {
        let mut line = format!("- {}", term.word);
        let sung: Vec<String> = term
            .sung
            .iter()
            .map(|one| {
                if crate::words::plain(&one.written) == crate::words::plain(&term.word) {
                    one.sung.clone()
                } else {
                    format!("{} → {}", one.written, one.sung)
                }
            })
            .collect();
        if !sung.is_empty() {
            line.push_str(&format!(" (sung: {})", sung.join(", ")));
        }
        line
    };
    let fresh = |term: &Term| term.bank == Some(Bank::Fresh);

    if let Some(named) = block {
        let Some(found) = blocks::find(conn, profile_id, named)? else {
            return Ok(format!("(the bank has no block called “{named}”)"));
        };
        let lines: Vec<String> = blocks::words(conn, &found.id)?
            .iter()
            .filter(|term| fresh(term))
            .map(said)
            .collect();
        if lines.is_empty() {
            return Ok(format!("(the block “{}” holds no fresh words)", found.name));
        }
        return Ok(format!(
            "Words the author keeps for songs to come - the block “{}”. Use them where they \
             fit, sung as written here:\n{}",
            found.name,
            lines.join("\n")
        ));
    }

    let terms = super::list(conn, profile_id)?;
    let views = blocks::views(conn, profile_id)?;
    let mut out = String::from(
        "Words the author keeps for songs to come, by block. Use them where they fit, sung as \
         written here:\n",
    );
    let mut any = false;
    let mut placed: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for view in &views {
        let lines: Vec<String> = view
            .term_ids
            .iter()
            .filter_map(|id| terms.iter().find(|term| term.id == *id))
            .filter(|term| fresh(term))
            .map(said)
            .collect();
        placed.extend(view.term_ids.iter().map(String::as_str));
        if !lines.is_empty() {
            any = true;
            out.push_str(&format!("\n{}:\n{}\n", view.block.name, lines.join("\n")));
        }
    }
    let loose: Vec<String> = terms
        .iter()
        .filter(|term| fresh(term) && !placed.contains(term.id.as_str()))
        .map(said)
        .collect();
    if !loose.is_empty() {
        any = true;
        out.push_str(&format!("\nIn no block:\n{}\n", loose.join("\n")));
    }
    if !any {
        return Ok("(the bank of words holds no fresh words)".to_owned());
    }
    Ok(out.trim_end().to_owned())
}

/// The neighbours for `{neighbours}`: each work's title, kind, the words it
/// shares, and its text.
pub fn neighbours(found: &[Neighbour]) -> String {
    if found.is_empty() {
        return "(no other work shares this text's words)".to_owned();
    }
    let mut out = String::from(
        "The works whose words stand closest to this text - by shared words, which is where \
         to look, not yet a judgement of meaning:\n",
    );
    for one in found {
        let mut body: String = one.body.chars().take(NEIGHBOUR_TEXT).collect();
        if body.len() < one.body.len() {
            body.push_str(" …");
        }
        out.push_str(&format!(
            "\n### “{}” ({}) - shares: {}\n\n{}\n",
            one.title,
            one.kind,
            one.shared.join(", "),
            body.trim()
        ));
    }
    out.trim_end().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::register::tests::song;
    use crate::register::{self, NewTerm, TermKind};

    #[test]
    fn the_sheet_groups_by_strictness_and_says_what_the_text_takes() {
        let (conn, profile_id) = fixtures::workspace();
        song(&conn, &profile_id, "One", "dust on the sill");
        register::create(
            &conn,
            &profile_id,
            NewTerm {
                word: "dust".into(),
                strictness: Some(Strictness::Ban),
                note: Some("everywhere".into()),
                ..NewTerm::default()
            },
        )
        .unwrap();
        register::create(
            &conn,
            &profile_id,
            NewTerm {
                word: "a lighthouse nobody keeps".into(),
                kind: Some(TermKind::Image),
                ..NewTerm::default()
            },
        )
        .unwrap();

        let sheet = register(&conn, &profile_id, Some("dust, dust")).unwrap();

        assert!(
            sheet.contains("Banned - do not use:\n- dust (1) - everywhere"),
            "{sheet}"
        );
        assert!(
            sheet.contains("- a lighthouse nobody keeps (0) [limit]"),
            "{sheet}"
        );
        assert!(
            sheet.contains("already takes, word for word: dust ×2 (ban)"),
            "{sheet}"
        );
        assert!(
            !sheet.contains("Limited"),
            "an empty group is left out: {sheet}"
        );
    }

    #[test]
    fn an_empty_register_says_so() {
        let (conn, profile_id) = fixtures::workspace();
        assert_eq!(
            register(&conn, &profile_id, None).unwrap(),
            "(the register of repeats is empty)"
        );
    }
}
