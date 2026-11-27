//! Writing progress (`progress`) from small in-memory games.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used)]

use larp_graph::{StatusCount, progress};
use larp_model::{Game, SourceFile, load};

fn game(files: &[(&str, &str)]) -> Game {
    let files: Vec<SourceFile> = files
        .iter()
        .map(|(path, text)| SourceFile {
            path: (*path).to_owned(),
            text: (*text).to_owned(),
        })
        .collect();
    let checked = load(&files);
    assert!(checked.game.is_some(), "{:?}", checked.diagnostics);
    checked.game.unwrap()
}

const GAME: (&str, &str) = ("game.toml", "[game]\ntitle = \"T\"\nlanguage = \"cs\"\n");

fn tracked() -> Game {
    game(&[
        GAME,
        (
            "people/ann.toml",
            "[person]\nname = \"Ann\"\nrole = \"player\"\nstatus = \"done\"\nwriter = \"Jana\"\n",
        ),
        (
            "people/bob.toml",
            "[person]\nname = \"Bob\"\nrole = \"npc\"\n",
        ),
        (
            "actions/b-zed.toml",
            "[action]\ntitle = \"Zed\"\nowners = [\"ann\"]\nstatus = \"draft\"\nwriter = \"Jana\"\n",
        ),
        (
            "actions/a-alfa.toml",
            "[action]\ntitle = \"Alfa\"\nowners = [\"ann\"]\nstatus = \"review\"\nwriter = \"Petr\"\n",
        ),
        (
            "actions/c-beta.toml",
            "[decision]\ntitle = \"Beta\"\nowners = [\"bob\"]\nstatus = \"draft\"\n\n[[decision.option]]\nlabel = \"x\"\n",
        ),
        (
            "actions/d-done.toml",
            "[action]\ntitle = \"Hotovo\"\nowners = [\"bob\"]\nstatus = \"done\"\n",
        ),
    ])
}

#[test]
fn counts_nodes_and_people_by_status() {
    let p = progress(&tracked());
    assert_eq!(
        p.all,
        StatusCount {
            draft: 2,
            review: 1,
            done: 2,
            untracked: 1,
        }
    );
    assert_eq!(p.all.total(), 6);
    assert!(!p.is_untracked());
}

#[test]
fn writers_are_counted_by_name_and_parts_without_writer_are_left_out() {
    let p = progress(&tracked());
    let names: Vec<&str> = p.writers.iter().map(|(w, _)| w.as_str()).collect();
    assert_eq!(names, ["Jana", "Petr"]);
    assert_eq!(
        p.writers[0].1,
        StatusCount {
            draft: 1,
            review: 0,
            done: 1,
            untracked: 0,
        }
    );
    assert_eq!(p.writers[1].1.review, 1);
}

#[test]
fn open_parts_are_drafts_then_reviews_each_by_title() {
    let p = progress(&tracked());
    let open: Vec<(&str, &str, &str)> = p
        .open
        .iter()
        .map(|o| (o.status, o.title.as_str(), o.id.as_str()))
        .collect();
    assert_eq!(
        open,
        [
            ("draft", "Beta", "c-beta"),
            ("draft", "Zed", "b-zed"),
            ("review", "Alfa", "a-alfa"),
        ]
    );
    assert_eq!(p.open[1].writer.as_deref(), Some("Jana"));
    assert_eq!(p.open[0].writer, None);
    assert!(p.open.iter().all(|o| o.kind == "node"));
}

#[test]
fn people_in_draft_are_open_parts_too() {
    let g = game(&[
        GAME,
        (
            "people/ann.toml",
            "[person]\nname = \"Ann\"\nrole = \"player\"\nstatus = \"draft\"\n",
        ),
    ]);
    let p = progress(&g);
    assert_eq!(p.open.len(), 1);
    assert_eq!(p.open[0].kind, "person");
    assert_eq!(p.open[0].title, "Ann");
}

#[test]
fn a_game_without_any_status_is_untracked() {
    let g = game(&[
        GAME,
        (
            "people/ann.toml",
            "[person]\nname = \"Ann\"\nrole = \"player\"\n",
        ),
        (
            "actions/a.toml",
            "[action]\ntitle = \"A\"\nowners = [\"ann\"]\n",
        ),
    ]);
    let p = progress(&g);
    assert!(p.is_untracked());
    assert_eq!(p.all.untracked, 2);
    assert!(p.writers.is_empty() && p.open.is_empty());
}

#[test]
fn an_unknown_status_is_a_load_error() {
    let files = [
        SourceFile {
            path: "game.toml".into(),
            text: GAME.1.into(),
        },
        SourceFile {
            path: "actions/a.toml".into(),
            text: "[action]\ntitle = \"A\"\nstatus = \"finished\"\n".into(),
        },
    ];
    assert!(load(&files).game.is_none());
}
