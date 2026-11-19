//! The planned slots of works (v0.138, FR-11).

use tauri::State;

use super::active;
use crate::actions;
use crate::error::Result;
use crate::state::AppState;
use crate::work_plan::{self, NewWorkPlan, WorkPlan, WorkPlanPatch};

/// The open slots of the active profile, earliest first.
#[tauri::command]
pub fn work_plans(state: State<'_, AppState>) -> Result<Vec<WorkPlan>> {
    let conn = state.conn();
    let profile_id = active(&conn)?;
    let config = crate::profile::config_for(&conn, &profile_id)?;
    work_plan::open(&conn, &profile_id, config.plan_window())
}

/// The days of a period by the profile's rhythm, before anything is made
/// (v0.152, FR-20 20.3).
#[tauri::command]
pub fn plan_period(
    state: State<'_, AppState>,
    from: String,
    months: u32,
) -> Result<crate::period::PeriodPlan> {
    let conn = state.conn();
    let profile_id = active(&conn)?;
    crate::period::plan(&conn, &profile_id, &from, months)
}

/// A period laid out: its months' collections and its days' drafts, planned
/// (v0.152).
#[tauri::command]
pub fn lay_out_period(
    state: State<'_, AppState>,
    layout: actions::period::PeriodLayout,
) -> Result<actions::period::PeriodMade> {
    actions::period::apply(&state.conn(), layout)
}

#[tauri::command]
pub fn plan_work(state: State<'_, AppState>, plan: NewWorkPlan) -> Result<WorkPlan> {
    actions::work_plan::plan(&state.conn(), plan)
}

#[tauri::command]
pub fn update_work_plan(
    state: State<'_, AppState>,
    id: String,
    patch: WorkPlanPatch,
) -> Result<WorkPlan> {
    actions::work_plan::update(&state.conn(), &id, patch)
}

#[tauri::command]
pub fn delete_work_plan(state: State<'_, AppState>, id: String) -> Result<()> {
    actions::work_plan::delete(&state.conn(), &id)
}

#[tauri::command]
pub fn fulfill_work_plan(
    state: State<'_, AppState>,
    id: String,
    kind: Option<String>,
) -> Result<actions::work_plan::Fulfilled> {
    actions::work_plan::fulfill(&state.conn(), &id, kind.as_deref())
}
