//! Where a proposal goes: into a chat a person reads, never into the work.

use rusqlite::Connection;

use super::Session;
use crate::assistant::apply;
use crate::assistant::proposal::Proposal;
use crate::error::{Error, Result};
use crate::journal::{self, Record};
use crate::{assistant, work};

/// How the chat a proposal lands in is named when the client did not say
/// who it is.
pub(super) const UNNAMED_CLIENT: &str = "An agent";

/// Put a proposal where a person will see it: an assistant message in the
/// chat named after the client, on the work when there is one.
///
/// The chat is the client's, one per work, so a week of proposals from one
/// agent reads as one conversation rather than twenty chats named
/// "Proposal". The message body is the proposed text itself for a version —
/// that is what *insert as version* keeps, verbatim — and the reasoning goes
/// beside it in `meta.note`. A line in the journal says it arrived, so the
/// bell in the corner counts it.
pub(super) fn deliver(
    conn: &Connection,
    profile_id: &str,
    session: &Session,
    work: Option<&work::Work>,
    proposal: Proposal,
    body: &str,
    note: Option<&str>,
) -> Result<assistant::Message> {
    // Comments are read and answered inside the window, from where a
    // screenshot was pasted or a comment was opened; no tool proposes them,
    // and the sentence below has no line for one.
    if matches!(
        proposal,
        Proposal::Comment { .. } | Proposal::Reply { .. } | Proposal::Description { .. }
    ) {
        return Err(Error::refused("mcp.commentsNotProposable"));
    }
    let client = session
        .client
        .clone()
        .unwrap_or_else(|| UNNAMED_CLIENT.to_owned());
    let work_id = work.map(|w| w.id.as_str());
    let message = apply::into_chat(conn, profile_id, &client, work_id, &proposal, body, note)?;

    // Literal keys, one per sentence in the locale: the journal gate reads
    // them out of the source. A note on nothing has no title to name, and a
    // sentence with an empty quotation in it reads as a bug, so it gets a
    // sentence of its own. A work that does not exist yet has no row to file
    // the line under; the title it would have is the one thing to name. The
    // arms stay close under the `let`: the gate finds a bound record by it.
    let record = match (&proposal, work) {
        (Proposal::Version { .. }, _) => Record::new("proposal.version"),
        (Proposal::Score { .. }, _) => Record::new("proposal.score"),
        (Proposal::Note { .. }, Some(_)) => Record::new("proposal.note"),
        (Proposal::Note { .. }, None) => Record::new("proposal.freeNote"),
        (Proposal::Notes { .. }, Some(_)) => Record::new("proposal.notes"),
        (Proposal::Notes { .. }, None) => Record::new("proposal.freeNotes"),
        (Proposal::Work { .. }, Some(_)) => Record::new("proposal.package"),
        (Proposal::Scenes { .. }, _) => Record::new("proposal.scenes"),
        (Proposal::Release { .. }, _) => Record::new("proposal.release"),
        (Proposal::CoverIdeas { .. }, _) => Record::new("proposal.coverIdeas"),
        (Proposal::Trials { .. }, _) => Record::new("proposal.trials"),
        (Proposal::Cards { .. }, _) => Record::new("proposal.cards"),
        (Proposal::Month { .. }, _) => Record::new("proposal.month"),
        (Proposal::Media { .. }, _) => Record::new("proposal.media"),
        (Proposal::Work { title, .. }, None) => {
            Record::new("proposal.work").param("title", title.clone().unwrap_or_default())
        }
        (Proposal::Words { .. }, _) => Record::new("proposal.words"),
        (Proposal::Styles { .. }, _) => Record::new("proposal.styles"),
        (Proposal::Short { .. }, Some(_)) => Record::new("proposal.short"),
        (Proposal::Short { .. }, None) => Record::new("proposal.shortStyle"),
        (Proposal::Canon { .. }, Some(_)) => Record::new("proposal.canon"),
        (Proposal::Canon { .. }, None) => Record::new("proposal.freeCanon"),
        // Refused at the top of this function.
        (Proposal::Comment { .. } | Proposal::Reply { .. } | Proposal::Description { .. }, _) => {
            return Err(Error::refused("mcp.commentsNotProposable"));
        }
        // A card's description alone is the action on the card, which knows
        // the facts it answers to; an agent proposes one inside a package for
        // the canon (`propose_canon` descriptions), read with the card.
        (Proposal::CardPrompt { .. } | Proposal::CardNote { .. }, _) => {
            return Err(Error::refused("mcp.cardPromptNotProposable"));
        }
        // Bricks are proposed by the dictionary's own "Explain", which read
        // the phrases against it; no tool sends a package of them.
        (Proposal::Bricks { .. }, _) => {
            return Err(Error::refused("mcp.bricksNotProposable"));
        }
    };
    let mut record = record.param("client", client);
    if let Some(w) = work {
        record = record
            .param("title", w.title.clone())
            .about("work", w.id.clone());
    }
    journal::record(conn, profile_id, record);

    Ok(message)
}
