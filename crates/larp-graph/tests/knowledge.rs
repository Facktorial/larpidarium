//! `GraphData.knowledge`: who knows which info and from when, for the Knowledge tab.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used)]

use larp_graph::{KnowView, build};
use larp_model::{Game, SourceFile, load};

const GAME: &str = "[game]\ntitle = \"Test\"\nlanguage = \"cs\"\n";

fn game(files: &[(&str, &str)]) -> Game {
    let files: Vec<SourceFile> = std::iter::once(("game.toml", GAME))
        .chain(files.iter().copied())
        .map(|(path, text)| SourceFile {
            path: path.to_owned(),
            text: text.to_owned(),
        })
        .collect();
    let checked = load(&files);
    assert!(checked.game.is_some(), "{:?}", checked.diagnostics);
    checked.game.unwrap()
}

/// Ann starts with `heslo`; Bob finds `stopa` at an action; Ann is told it at a decision option.
fn knowledge() -> Vec<KnowView> {
    let g = game(&[
        (
            "people/ann.toml",
            "[person]\nname = \"Anna Růžičková\"\nrole = \"npc\"\n",
        ),
        (
            "people/bob.toml",
            "[person]\nname = \"Bob Řezníček\"\nrole = \"npc\"\n",
        ),
        (
            "segments.toml",
            "[[segment]]\nid = \"rano\"\nname = \"Ráno\"\n\n[[segment]]\nid = \"vecer\"\nname = \"Večer\"\n",
        ),
        (
            "info.toml",
            "[[info]]\nid = \"heslo\"\nname = \"Heslo\"\nstarting = true\nowners = [\"ann\"]\nending = true\n\n\
             [[info]]\nid = \"stopa\"\nname = \"Stopa\"\n",
        ),
        (
            "actions/nalez.toml",
            "[action]\ntitle = \"Nález\"\nsegment = \"rano\"\nowners = [\"bob\"]\ngives = [\"stopa\"]\n",
        ),
        (
            "actions/volba.toml",
            "[decision]\ntitle = \"Volba\"\nsegment = \"vecer\"\nowners = [\"ann\"]\n\n\
             [[decision.option]]\nlabel = \"Přiznat se\"\nneeds = [\"stopa\"]\n\n\
             [[decision.option]]\nlabel = \"Mlčet\"\n",
        ),
    ]);
    build(&g, &[], None).knowledge
}

fn row(k: &KnowView) -> (&str, &str, &str, Option<&str>, Option<&str>) {
    (
        k.person.as_str(),
        k.info.as_str(),
        k.how,
        k.node.as_deref(),
        k.option.as_deref(),
    )
}

#[test]
fn lists_every_way_with_how_node_and_option() {
    let k = knowledge();
    let rows: Vec<_> = k.iter().map(row).collect();
    assert_eq!(
        rows,
        vec![
            ("ann", "heslo", "start", None, None),
            ("ann", "stopa", "told", Some("volba"), Some("Přiznat se")),
            ("bob", "stopa", "finds", Some("nalez"), None),
        ]
    );
}

#[test]
fn is_empty_without_info() {
    let g = game(&[(
        "people/ann.toml",
        "[person]\nname = \"Ann\"\nrole = \"npc\"\n",
    )]);
    assert!(build(&g, &[], None).knowledge.is_empty());
}
