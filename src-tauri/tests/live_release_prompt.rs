//! A release's writer, composed against a real workspace, by hand.
//!
//! Ignored by default: it needs a workspace to point at, which CI has none of.
//! `KILNA_LIVE_DB` points at a *copy* of one; the workspace is opened the way
//! the window opens it - migrations, then the profile's upgrade - so what is
//! checked is what the owner's copy of the profile says after an update, not
//! what the shipped one says. `KILNA_LIVE_RELEASE` names the release, and
//! `KILNA_LIVE_OUT` is a directory the prompt and the method are written to,
//! for a trial run of the method by hand; nothing is printed of the text.

#[test]
#[ignore]
fn a_written_release_is_asked_for_another_take() {
    let (Ok(path), Ok(release_id)) = (
        std::env::var("KILNA_LIVE_DB"),
        std::env::var("KILNA_LIVE_RELEASE"),
    ) else {
        panic!(
            "set KILNA_LIVE_DB to a copy of a real workspace and KILNA_LIVE_RELEASE to a release in it"
        );
    };
    let conn = kilna_lib::db::open(std::path::Path::new(&path)).unwrap();
    kilna_lib::profile::seed(&conn).unwrap();

    let (composed, _) =
        kilna_lib::assistant::task::compose_for_release(&conn, &release_id, "release-meta")
            .unwrap();
    let method = composed.method.clone().unwrap_or_default();

    assert!(
        method.contains("## An earlier take"),
        "the owner's untouched method followed the shipped one"
    );
    assert!(
        composed.prompt.contains("earlier take"),
        "a release with words is asked for another take"
    );
    if let Ok(out) = std::env::var("KILNA_LIVE_OUT") {
        let out = std::path::Path::new(&out);
        std::fs::write(out.join("prompt.txt"), &composed.prompt).unwrap();
        std::fs::write(out.join("method.md"), &method).unwrap();
    }
    println!(
        "prompt {} characters, method {} characters",
        composed.prompt.chars().count(),
        method.chars().count()
    );
}
