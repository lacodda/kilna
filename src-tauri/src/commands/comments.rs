//! The audience's comments, the channels they came through, and the replies.

use tauri::State;

use super::active;
use crate::actions;
use crate::comment::{self, Comment, CommentFilter, CommentPatch, NewComment};
use crate::error::Result;
use crate::state::AppState;
use crate::trash::Entity;

#[tauri::command]
pub fn list_comments(
    state: State<'_, AppState>,
    filter: Option<CommentFilter>,
) -> Result<Vec<Comment>> {
    let conn = state.conn();
    comment::list(&conn, &active(&conn)?, &filter.unwrap_or_default())
}

/// Every channel comments came through, with how many still wait on each.
#[tauri::command]
pub fn comment_channels(state: State<'_, AppState>) -> Result<Vec<(String, i64)>> {
    let conn = state.conn();
    comment::channels(&conn, &active(&conn)?)
}

#[tauri::command]
pub fn create_comment(state: State<'_, AppState>, comment: NewComment) -> Result<Comment> {
    actions::comment::create(&state.conn(), comment)
}

#[tauri::command]
pub fn update_comment(
    state: State<'_, AppState>,
    id: String,
    patch: CommentPatch,
) -> Result<Comment> {
    actions::comment::update(&state.conn(), &id, patch)
}

#[tauri::command]
pub fn delete_comment(state: State<'_, AppState>, id: String) -> Result<String> {
    actions::trash::discard(&state.conn(), Entity::Comment, &id)
}
