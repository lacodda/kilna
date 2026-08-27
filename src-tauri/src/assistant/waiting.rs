//! Telling "it is done" apart from "it stopped to ask".
//!
//! A background run ends the same way whichever it was: the process exits and
//! an answer lands in a chat nobody is looking at. If that answer was a
//! question, it sits there unanswered — and the work it was about waits with
//! it. So a finished task is read once more to ask: does this end the exchange
//! or hand it back?
//!
//! Two detectors, because either alone is wrong in a way the other covers.
//! The marker is exact and the assistant is told to use it, but instructions
//! get ignored. The heuristic catches what the marker missed, and pays for it
//! with the occasional false positive. That trade is deliberate: a banner you
//! dismiss with one click costs a glance, a question you never noticed costs
//! the hour the work sat still.

/// What a run's last word turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
    /// It answered and expects nothing back.
    Settled,
    /// It asked something. The exchange is open.
    Waiting,
}

/// The marker a task's prompt asks the assistant to end with when it needs an
/// answer.
///
/// Written in English like everything the model is told, and deliberately
/// unlovely: it has to survive being echoed inside an answer without ever
/// being written by accident.
pub const MARKER: &str = "[WAITING FOR ANSWER]";

/// What is appended to a task's prompt so the assistant can mark its own
/// question.
///
/// Only tasks carry this. A prompt typed in the panel is read by the person
/// sending it — adding an instruction they did not write would break the rule
/// that the composer shows exactly what goes out (v0.28), and the banner would
/// be telling them about a question already on their screen.
pub const INSTRUCTION: &str = "\n\nIf you need an answer from me before you can finish, end your reply with the line [WAITING FOR ANSWER] on its own. Do not write that line when you are simply reporting what you did.";

/// Add the marker instruction to a task's prompt.
pub fn instruct(prompt: &str) -> String {
    format!("{prompt}{INSTRUCTION}")
}

/// Read a finished answer and decide whether it is waiting on the reader.
pub fn read(body: &str) -> Ending {
    if carries_marker(body) || asks_a_question(body) {
        Ending::Waiting
    } else {
        Ending::Settled
    }
}

/// Whether the assistant marked its own answer.
///
/// Looked for near the end rather than anywhere: an answer discussing the
/// convention — "end with [WAITING FOR ANSWER] when you need me" — mentions the
/// marker without being a question. The instruction asks for a final line, so
/// that is where it counts.
fn carries_marker(body: &str) -> bool {
    body.lines().rev().take(3).any(|line| line.trim() == MARKER)
}

/// Whether the answer *ends* by asking rather than by reporting.
///
/// What decides is the last thing said. An answer may raise a question and then
/// settle it — "Should the verse stay? I think yes, and I kept it" — and that is
/// a report; reading any of the closing lines would call it a question and put
/// a banner on finished work.
///
/// The one thing allowed past the end is a short aside beneath the question — a
/// parenthetical, a one-line note. It is skipped by length, not by guessing at
/// its content: anything long enough to be a paragraph is the answer's real
/// ending and gets to decide.
fn asks_a_question(body: &str) -> bool {
    const ASIDE: usize = 60;

    let mut lines = body
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .rev()
        .peekable();

    // Step past short trailing asides, but never past more than two: a stack of
    // one-liners is a list, and the question would be too far back to be what
    // the answer left the reader with.
    let mut skipped = 0;
    while skipped < 2 {
        let Some(line) = lines.peek() else { break };
        if closes_by_asking(line) || line.chars().count() > ASIDE {
            break;
        }
        lines.next();
        skipped += 1;
    }

    lines.next().is_some_and(closes_by_asking)
}

/// Whether one line ends by handing the exchange back.
fn closes_by_asking(line: &str) -> bool {
    ends_in_a_question(line) || hands_it_back(line)
}

/// A line that ends by asking.
///
/// The question mark has to be the end of the line, not merely present: "the
/// second verse (why?) is the weak one" reports, it does not ask.
fn ends_in_a_question(line: &str) -> bool {
    line.ends_with('?') || line.ends_with("?**") || line.ends_with("?_")
}

/// Phrases that hand the decision back without a question mark.
///
/// Matched with an explicit right-hand boundary rather than a word-boundary
/// class: kilna's interface is Russian as well as English, and the lesson from
/// atlas is that `\b` is ASCII-only — "подтвердите" matched inside
/// "подтвердил", which is a report, not a request.
fn hands_it_back(line: &str) -> bool {
    const HANDOFFS: [&str; 10] = [
        "let me know",
        "tell me which",
        "say the word",
        "which would you",
        "confirm",
        "скажи",
        "подтверди",
        "выбери",
        "уточни",
        "дай знать",
    ];

    let lowered = line.to_lowercase();

    HANDOFFS.iter().any(|phrase| {
        lowered.match_indices(phrase).any(|(at, matched)| {
            let after = &lowered[at + matched.len()..];
            starts_a_new_word(after)
        })
    })
}

/// Whether what follows a matched phrase is not a continuation of the word.
///
/// This is the right-hand boundary the lesson is about. A letter — Latin or
/// Cyrillic — means the phrase was only the head of a longer word
/// ("подтвердил", "confirmed"), and a longer word is a different word.
fn starts_a_new_word(rest: &str) -> bool {
    match rest.chars().next() {
        None => true,
        Some(next) => !next.is_alphanumeric(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_marked_answer_is_waiting() {
        let body = "I rewrote the second verse.\n\n[WAITING FOR ANSWER]";

        assert_eq!(read(body), Ending::Waiting);
    }

    #[test]
    fn a_plain_report_is_settled() {
        let body = "I rewrote the second verse and tightened the chorus.";

        assert_eq!(read(body), Ending::Settled);
    }

    #[test]
    fn a_closing_question_is_waiting_without_the_marker() {
        let body = "The second verse could go two ways.\n\nWhich one do you want?";

        assert_eq!(read(body), Ending::Waiting);
    }

    #[test]
    fn a_question_mark_inside_a_sentence_is_not_a_question() {
        let body = "The line asks \"where now?\" and never answers it. That is the weakness.";

        assert_eq!(
            read(body),
            Ending::Settled,
            "a report quoting a question is still a report"
        );
    }

    #[test]
    fn a_question_answered_further_down_is_settled() {
        let body = "Should the verse stay?\n\nI think yes, and I kept it. Here is why: it carries \
                    the only image the chorus reuses.";

        assert_eq!(
            read(body),
            Ending::Settled,
            "what decides is how the answer ends, not what it raised on the way"
        );
    }

    #[test]
    fn a_handoff_phrase_counts_without_a_question_mark() {
        let body = "I can cut the bridge or rewrite it. Let me know which.";

        assert_eq!(read(body), Ending::Waiting);
    }

    #[test]
    fn a_russian_handoff_counts() {
        let body = "Готово. Скажи, оставить ли третий куплет.";

        assert_eq!(read(body), Ending::Waiting);
    }

    /// The lesson from atlas, as a test: the right-hand boundary matters, and
    /// `\b`-style matching gets it wrong on Cyrillic.
    #[test]
    fn a_past_tense_report_is_not_a_request() {
        let body = "Я подтвердил размер и оставил всё как есть.";

        assert_eq!(
            read(body),
            Ending::Settled,
            "подтверди|л is a report; matching inside a longer word would call it a question"
        );
    }

    #[test]
    fn an_english_past_tense_report_is_not_a_request() {
        let body = "I confirmed the metre against the chorus and left it alone.";

        assert_eq!(read(body), Ending::Settled);
    }

    #[test]
    fn a_handoff_at_the_very_end_of_the_text_counts() {
        let body = "Оставить или убрать — скажи";

        assert_eq!(
            read(body),
            Ending::Waiting,
            "a phrase ending the text has nothing after it, which is still a boundary"
        );
    }

    #[test]
    fn the_marker_is_read_at_the_end_not_in_passing() {
        let body = "You asked me to end with [WAITING FOR ANSWER] when I need you. I did not \
                    need you this time, so here is the rewrite in full.\n\nThe cranes go still.";

        assert_eq!(
            read(body),
            Ending::Settled,
            "an answer explaining the convention is not using it"
        );
    }

    #[test]
    fn a_marker_below_a_closing_note_still_counts() {
        let body =
            "Which verse should go?\n\n[WAITING FOR ANSWER]\n\n(asked before I cut anything)";

        assert_eq!(read(body), Ending::Waiting);
    }

    #[test]
    fn a_question_above_a_short_sign_off_still_counts() {
        let body =
            "Both readings work.\n\nWhich do you prefer?\n\nEither way it is a small change.";

        assert_eq!(read(body), Ending::Waiting);
    }

    #[test]
    fn an_empty_answer_is_settled() {
        assert_eq!(read(""), Ending::Settled);
        assert_eq!(read("   \n\n  "), Ending::Settled);
    }

    #[test]
    fn a_bold_question_counts() {
        let body = "I see two options.\n\n**Which one do you want?**";

        assert_eq!(read(body), Ending::Waiting);
    }

    #[test]
    fn the_instruction_is_appended_whole() {
        let instructed = instruct("Critique the lyrics");

        assert!(instructed.starts_with("Critique the lyrics"));
        assert!(
            instructed.contains(MARKER),
            "the instruction must name the marker it asks for"
        );
    }
}
