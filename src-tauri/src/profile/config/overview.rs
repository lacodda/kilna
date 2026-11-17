//! The card's overview: which widgets stand on it, and how they are laid out.

use serde::{Deserialize, Serialize};

use crate::error::Reason;

use super::*;

/// The names [`OverviewConfig::layout`] may take: an even grid, a lead
/// column with a rail beside it, full-width bands, a mosaic, a sheet of rows.
pub const OVERVIEW_LAYOUTS: [&str; 5] = ["grid", "lead", "bands", "mosaic", "sheet"];

/// The sizes a widget may take: one cell, two across, two by two.
pub const WIDGET_SIZES: [&str; 3] = ["s", "m", "l"];

/// A work's overview: which of the layouts it is drawn in, and the widgets on it.
///
/// The layout and the sizes are words rather than enums on this side, and
/// checked by [`ProfileConfig::validate`] instead of by the parser. The two
/// ends of the document are held to different rules: a save that names a
/// layout nobody draws is a mistake to refuse with its name, while a stored
/// document written by a later build - a sixth layout, a new size - must
/// still read, or the whole profile would fail to load over one word the
/// window can fall back from. Nothing in the backend draws a board, so a word
/// costs it nothing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ts_rs::TS)]
pub struct OverviewConfig {
    /// One of [`OVERVIEW_LAYOUTS`].
    pub layout: String,
    /// The widgets, each with its size and place. Empty means the placement
    /// the window ships, so choosing a layout never has to invent one.
    #[serde(default)]
    pub widgets: Vec<WidgetPlacement>,
}

/// Where one widget stands on the overview, and how large it is.
///
/// The id is the window's widget catalogue's - `score`, `text`, `releases`
/// and the rest - and it is not checked against a list here: a widget this
/// build does not know, written by a later one, is kept on save and simply
/// not drawn, the way an unknown catalogue column is.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ts_rs::TS)]
pub struct WidgetPlacement {
    pub id: String,
    /// One of [`WIDGET_SIZES`].
    pub size: String,
    /// The widget's place in the layout's reading order, first at 0.
    pub position: u32,
}

impl OverviewConfig {
    /// What is wrong with the overview as written, in the words of
    /// [`ProfileConfig::problems`].
    pub(super) fn validate_into(&self, problems: &mut Vec<Reason>) {
        if !OVERVIEW_LAYOUTS.contains(&self.layout.as_str()) {
            let (head, last) = one_of(&OVERVIEW_LAYOUTS);
            problems.push(
                Reason::of("problem.overviewLayout")
                    .param("head", head)
                    .param("last", last)
                    .param("layout", self.layout.as_str()),
            );
        }
        // A widget named twice would be drawn twice, or once at a place
        // nobody chose - the corruption `unique` exists to catch.
        unique(
            problems,
            Reason::of("place.overviewWidgets"),
            self.widgets.iter().map(|widget| widget.id.clone()),
        );
        for (index, widget) in self.widgets.iter().enumerate() {
            if !WIDGET_SIZES.contains(&widget.size.as_str()) {
                let (head, last) = one_of(&WIDGET_SIZES);
                problems.push(
                    Reason::of("problem.widgetSize")
                        .param("place", widget_at(index, &widget.id))
                        .param("head", head)
                        .param("last", last)
                        .param("size", widget.size.as_str()),
                );
            }
        }
    }
}

/// The words a field may take, for a sentence that lists them the way a
/// person would, "`s`, `m` or `l`": all but the last, and the last, so the
/// sentence of each language puts its own "or" between them.
pub(super) fn one_of(words: &[&str]) -> (String, String) {
    let quoted: Vec<String> = words.iter().map(|word| format!("`{word}`")).collect();
    match quoted.split_last() {
        Some((last, rest)) => (rest.join(", "), last.clone()),
        None => (String::new(), String::new()),
    }
}
