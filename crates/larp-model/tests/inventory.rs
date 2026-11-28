//! Preparing the game: `prop`, `quantity`, `location` and `handout` of items and info, as
//! `docs/format.md` ("Items, rooms, info") describes them.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used, clippy::panic)]

use larp_model::text::{each_text, expand};
use larp_model::{Checked, Diagnostic, Game, Severity, SourceFile, load};

const GAME: &str = "[game]\ntitle = \"Opatství\"\nlanguage = \"cs\"\n";
const ROOMS: &str = "[[room]]\nid = \"library\"\nname = \"Knihovna\"\n";
const MARTA: &str = "[person]\nname = \"Marta Řeháková\"\nrole = \"player\"\n";

fn run(files: &[(&str, &str)]) -> Checked {
    let files: Vec<SourceFile> = std::iter::once(("game.toml", GAME))
        .chain(files.iter().copied())
        .map(|(path, text)| SourceFile {
            path: path.to_owned(),
            text: text.to_owned(),
        })
        .collect();
    load(&files)
}

fn game(files: &[(&str, &str)]) -> Game {
    let checked = run(files);
    let diagnostics = format!("{:?}", checked.diagnostics);
    checked
        .game
        .unwrap_or_else(|| panic!("game did not load: {diagnostics}"))
}

fn errors(checked: &Checked) -> Vec<&Diagnostic> {
    checked
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect()
}

/// `items.toml` with one starting item owned by Marta and the given extra lines.
fn item(extra: &str) -> String {
    format!(
        "[[item]]\nid = \"letter\"\nname = \"Dopis\"\nowners = [\"marta\"]\nstarting = true\n{extra}"
    )
}

/// `info.toml` with one starting info owned by Marta and the given extra lines.
fn info(extra: &str) -> String {
    format!(
        "[[info]]\nid = \"note\"\nname = \"Vzkaz\"\nowners = [\"marta\"]\nstarting = true\n{extra}"
    )
}

// ---------- loading ----------

#[test]
fn loads_every_preparation_key_of_an_item() {
    let items = item(
        "prop = \"obálka zapečetěná voskem\"\nquantity = 3\nlocation = \"library\"\n\
         handout = \"Excelence, píši Vám v tísni…\"\n",
    );
    let g = game(&[
        ("people/marta.toml", MARTA),
        ("rooms.toml", ROOMS),
        ("items.toml", &items),
    ]);
    let t = &g.items[0];
    assert_eq!(t.prop.as_deref(), Some("obálka zapečetěná voskem"));
    assert_eq!(t.quantity.as_ref().map(|q| *q.get_ref()), Some(3));
    assert_eq!(t.location.as_ref().unwrap().get_ref().as_str(), "library");
    assert_eq!(t.handout.as_deref(), Some("Excelence, píši Vám v tísni…"));
    assert_eq!(t.count(), 3);
}

#[test]
fn loads_every_preparation_key_of_an_info() {
    let info = info(
        "prop = \"kartička\"\nquantity = 2\nlocation = \"library\"\nhandout = \"Sejdeme se o půlnoci.\"\n",
    );
    let g = game(&[
        ("people/marta.toml", MARTA),
        ("rooms.toml", ROOMS),
        ("info.toml", &info),
    ]);
    let t = &g.info[0];
    assert_eq!(t.prop.as_deref(), Some("kartička"));
    assert_eq!(t.count(), 2);
    assert_eq!(t.location.as_ref().unwrap().get_ref().as_str(), "library");
    assert_eq!(t.handout.as_deref(), Some("Sejdeme se o půlnoci."));
}

#[test]
fn preparation_keys_are_optional() {
    let g = game(&[("people/marta.toml", MARTA), ("items.toml", &item(""))]);
    let t = &g.items[0];
    assert_eq!(t.prop, None);
    assert!(t.quantity.is_none());
    assert!(t.location.is_none());
    assert_eq!(t.handout, None);
}

#[test]
fn count_is_one_when_quantity_is_missing() {
    let g = game(&[("people/marta.toml", MARTA), ("items.toml", &item(""))]);
    assert_eq!(g.items[0].count(), 1);
}

#[test]
fn count_is_one_when_quantity_is_one() {
    let g = game(&[
        ("people/marta.toml", MARTA),
        ("items.toml", &item("quantity = 1\n")),
    ]);
    assert_eq!(g.items[0].count(), 1);
}

#[test]
fn count_accepts_the_largest_quantity() {
    let g = game(&[
        ("people/marta.toml", MARTA),
        ("items.toml", &item(&format!("quantity = {}\n", u32::MAX))),
    ]);
    assert_eq!(g.items[0].count(), u32::MAX);
}

#[test]
fn multiline_handout_keeps_czech_text_and_line_breaks() {
    let items = item("handout = \"\"\"\nŘádky:\nčerná ůzká cesta\n\"\"\"\n");
    let g = game(&[("people/marta.toml", MARTA), ("items.toml", &items)]);
    assert_eq!(
        g.items[0].handout.as_deref(),
        Some("Řádky:\nčerná ůzká cesta\n")
    );
}

// ---------- quantity ----------

#[test]
fn quantity_zero_is_an_error_pointing_at_the_number() {
    let items = item("quantity = 0\n");
    let checked = run(&[("people/marta.toml", MARTA), ("items.toml", &items)]);
    assert!(checked.game.is_none());
    let e = errors(&checked);
    assert_eq!(e.len(), 1, "{e:?}");
    assert_eq!(e[0].file, "items.toml");
    assert_eq!(e[0].message, "`quantity` must be at least 1");
    assert_eq!(
        e[0].help.as_deref(),
        Some("leave `quantity` out when one prop is enough")
    );
    assert_eq!(&items[e[0].span.clone().unwrap()], "0");
}

#[test]
fn quantity_zero_in_info_is_reported_in_info_toml() {
    let info = info("quantity = 0\n");
    let checked = run(&[("people/marta.toml", MARTA), ("info.toml", &info)]);
    let e = errors(&checked);
    assert_eq!(e.len(), 1, "{e:?}");
    assert_eq!(e[0].file, "info.toml");
    assert_eq!(&info[e[0].span.clone().unwrap()], "0");
}

#[test]
fn negative_quantity_is_an_error() {
    let checked = run(&[
        ("people/marta.toml", MARTA),
        ("items.toml", &item("quantity = -1\n")),
    ]);
    assert!(checked.game.is_none());
    assert!(
        errors(&checked).iter().any(|d| d.file == "items.toml"),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn quantity_that_is_not_a_number_is_an_error() {
    let checked = run(&[
        ("people/marta.toml", MARTA),
        ("items.toml", &item("quantity = \"dva\"\n")),
    ]);
    assert!(checked.game.is_none());
    assert!(
        errors(&checked).iter().any(|d| d.file == "items.toml"),
        "{:?}",
        checked.diagnostics
    );
}

// ---------- location ----------

#[test]
fn location_in_a_room_gives_no_diagnostic() {
    let checked = run(&[
        ("people/marta.toml", MARTA),
        ("rooms.toml", ROOMS),
        ("items.toml", &item("location = \"library\"\n")),
    ]);
    assert!(errors(&checked).is_empty(), "{:?}", checked.diagnostics);
    assert!(checked.game.is_some());
}

#[test]
fn unknown_location_suggests_the_closest_room() {
    let items = item("location = \"librarx\"\n");
    let checked = run(&[
        ("people/marta.toml", MARTA),
        ("rooms.toml", ROOMS),
        ("items.toml", &items),
    ]);
    assert!(checked.game.is_none());
    let e = errors(&checked);
    assert_eq!(e.len(), 1, "{e:?}");
    assert_eq!(e[0].file, "items.toml");
    assert_eq!(e[0].message, "unknown location `librarx`, expected a room");
    assert_eq!(e[0].help.as_deref(), Some("did you mean `library`?"));
    assert_eq!(&items[e[0].span.clone().unwrap()], "\"librarx\"");
}

#[test]
fn unknown_location_does_not_suggest_a_person() {
    // `mart` is close to the person `marta`, but a location must be a room.
    let checked = run(&[
        ("people/marta.toml", MARTA),
        ("items.toml", &item("location = \"mart\"\n")),
    ]);
    let e = errors(&checked);
    assert_eq!(e.len(), 1, "{e:?}");
    assert_eq!(e[0].help, None);
}

#[test]
fn location_that_is_a_person_is_a_wrong_kind_error() {
    let info = info("location = \"marta\"\n");
    let checked = run(&[("people/marta.toml", MARTA), ("info.toml", &info)]);
    assert!(checked.game.is_none());
    let e = errors(&checked);
    assert_eq!(e.len(), 1, "{e:?}");
    assert_eq!(e[0].file, "info.toml");
    assert_eq!(
        e[0].message,
        "`marta` is a person, but a location must be a room"
    );
    assert_eq!(&info[e[0].span.clone().unwrap()], "\"marta\"");
}

#[test]
fn location_that_is_an_item_is_a_wrong_kind_error() {
    let items = item("location = \"letter\"\n");
    let checked = run(&[("people/marta.toml", MARTA), ("items.toml", &items)]);
    assert_eq!(
        errors(&checked)
            .iter()
            .map(|d| d.message.as_str())
            .collect::<Vec<_>>(),
        ["`letter` is an item, but a location must be a room"]
    );
}

#[test]
fn location_that_is_not_an_id_is_an_error() {
    let checked = run(&[
        ("people/marta.toml", MARTA),
        ("rooms.toml", ROOMS),
        ("items.toml", &item("location = \"Knihovna\"\n")),
    ]);
    assert!(checked.game.is_none());
    assert!(
        errors(&checked).iter().any(|d| d.file == "items.toml"),
        "{:?}",
        checked.diagnostics
    );
}

// ---------- links in handouts ----------

#[test]
fn unknown_link_in_a_handout_is_an_error_with_a_suggestion() {
    let checked = run(&[
        ("people/marta.toml", MARTA),
        ("items.toml", &item("handout = \"Od [[mart]]\"\n")),
    ]);
    let e = errors(&checked);
    assert_eq!(e.len(), 1, "{e:?}");
    assert_eq!(e[0].file, "items.toml");
    assert_eq!(e[0].message, "unknown link [[mart]] in handout");
    assert_eq!(e[0].help.as_deref(), Some("did you mean [[marta]]?"));
}

#[test]
fn unknown_link_in_an_info_handout_is_reported_in_info_toml() {
    let checked = run(&[
        ("people/marta.toml", MARTA),
        ("info.toml", &info("handout = \"[[nikdo-takovy]]\"\n")),
    ]);
    let e = errors(&checked);
    assert_eq!(e.len(), 1, "{e:?}");
    assert_eq!(e[0].file, "info.toml");
    assert_eq!(e[0].message, "unknown link [[nikdo-takovy]] in handout");
}

#[test]
fn link_from_a_handout_to_a_room_is_fine() {
    let checked = run(&[
        ("people/marta.toml", MARTA),
        ("rooms.toml", ROOMS),
        ("items.toml", &item("handout = \"Čekám v [[library]].\"\n")),
    ]);
    assert!(
        checked
            .diagnostics
            .iter()
            .all(|d| d.severity != Severity::Error)
    );
}

#[test]
fn each_text_visits_a_handout_as_handout() {
    let g = game(&[
        ("people/marta.toml", MARTA),
        ("items.toml", &item("handout = \"Karta\"\n")),
        ("info.toml", &info("handout = \"Info karta\"\n")),
    ]);
    let mut seen = Vec::new();
    each_text(&g, |place, t| {
        if place.field == "handout" {
            seen.push((place.file.to_owned(), t.to_owned()));
        }
    });
    assert_eq!(
        seen,
        [
            ("items.toml".to_owned(), "Karta".to_owned()),
            ("info.toml".to_owned(), "Info karta".to_owned()),
        ]
    );
}

#[test]
fn expand_replaces_links_in_handouts_and_keeps_the_original() {
    let blocks = "[[block]]\nid = \"datum\"\ntext = \"13. března\"\n";
    let g = game(&[
        ("people/marta.toml", MARTA),
        ("rooms.toml", ROOMS),
        ("blocks.toml", blocks),
        (
            "items.toml",
            &item("handout = \"[[marta]] píše dne [[datum]] z [[library]].\"\n"),
        ),
        ("info.toml", &info("handout = \"[[datum]]\"\n")),
    ]);
    let shown = expand(&g);
    assert_eq!(
        shown.items[0].handout.as_deref(),
        Some("Marta Řeháková píše dne 13. března z Knihovna.")
    );
    assert_eq!(shown.info[0].handout.as_deref(), Some("13. března"));
    assert_eq!(
        g.items[0].handout.as_deref(),
        Some("[[marta]] píše dne [[datum]] z [[library]].")
    );
}

#[test]
fn block_used_only_in_a_handout_is_not_unused() {
    let blocks = "[[block]]\nid = \"datum\"\ntext = \"13. března\"\n";
    let checked = run(&[
        ("people/marta.toml", MARTA),
        ("blocks.toml", blocks),
        ("items.toml", &item("handout = \"[[datum]]\"\n")),
    ]);
    assert!(
        !checked
            .diagnostics
            .iter()
            .any(|d| d.message.contains("datum")),
        "{:?}",
        checked.diagnostics
    );
}
