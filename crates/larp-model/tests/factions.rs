//! Factions: `factions.toml`, `Person.factions`, `Game::involves`, `Game::members` and the checks
//! that know about factions.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used, clippy::panic)]

use larp_model::model::Status;
use larp_model::{Checked, Diagnostic, Game, Id, Severity, SourceFile, load};

const GAME: &str = "[game]\ntitle = \"Test\"\nlanguage = \"cs\"\n";

const FACTIONS: &str = "[[faction]]\nid = \"montekove\"\nname = \"Montekové\"\ncolor = \"#2c3e50\"\n\
    text = \"Drží přístav.\"\ngoals = [\"Udržet přístav.\"]\nsecrets = [\"Otec prodal sklad.\"]\n\
    status = \"review\"\nwriter = \"Petra\"\n\n[faction.sheet]\nhistory = \"Od dob prvního majáku.\"\n";

/// A player with a complete sheet, so only the checks under test can warn about them.
fn player(name: &str, factions: &str) -> String {
    format!(
        "[person]\nname = \"{name}\"\nrole = \"player\"\nfactions = [{factions}]\ngoals = [\"Přežít\"]\n\n\
         [person.sheet]\nbackstory = \"b\"\nwhereabouts = \"w\"\n"
    )
}

/// An NPC, so that player checks stay quiet.
fn npc(name: &str, factions: &str) -> String {
    format!("[person]\nname = \"{name}\"\nrole = \"npc\"\nfactions = [{factions}]\n")
}

/// A decision owned by `owners` (a TOML list body such as `"\"romeo\""`).
fn decision(owners: &str) -> String {
    format!(
        "[decision]\ntitle = \"Rozhodnutí\"\nowners = [{owners}]\n\n[[decision.option]]\nlabel = \"Ano\"\n"
    )
}

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

fn with(checked: &Checked, severity: Severity) -> Vec<&Diagnostic> {
    checked
        .diagnostics
        .iter()
        .filter(|d| d.severity == severity)
        .collect()
}

fn messages(checked: &Checked, severity: Severity) -> Vec<&str> {
    with(checked, severity)
        .into_iter()
        .map(|d| d.message.as_str())
        .collect()
}

fn id(s: &str) -> Id {
    Id::parse(s).unwrap_or_else(|e| panic!("{s}: {e}"))
}

/// Romeo (player) and Tybalt (NPC) are Montekové; Julie (player) and the harbor room are not.
/// The faction owns `boj`, Romeo owns `rande`, the harbor owns `pristav-akce`.
fn verona() -> Game {
    game(&[
        ("factions.toml", FACTIONS),
        ("people/romeo.toml", &player("Romeo", "\"montekove\"")),
        ("people/tybalt.toml", &npc("Tybalt", "\"montekove\"")),
        ("people/julie.toml", &player("Julie Kapuletová", "")),
        (
            "rooms.toml",
            "[[room]]\nid = \"pristav\"\nname = \"Přístav\"\n",
        ),
        ("actions/boj.toml", &decision("\"montekove\"")),
        ("actions/rande.toml", &decision("\"romeo\", \"julie\"")),
        ("actions/pristav-akce.toml", &decision("\"pristav\"")),
    ])
}

fn owners_of<'a>(game: &'a Game, node: &str) -> &'a [larp_model::model::Ref] {
    game.nodes[&id(node)].owners()
}

// ---------- model: Faction, involves, members ----------

#[test]
fn loads_a_faction_with_every_key() {
    let g = verona();
    assert_eq!(g.factions.len(), 1);
    let f = &g.factions[0];
    assert_eq!(f.id.get_ref().as_str(), "montekove");
    assert_eq!(f.name, "Montekové");
    assert_eq!(
        f.color.as_ref().map(|c| c.get_ref().as_str()),
        Some("#2c3e50")
    );
    assert_eq!(f.text, "Drží přístav.");
    assert_eq!(f.goals, ["Udržet přístav."]);
    assert_eq!(f.secrets, ["Otec prodal sklad."]);
    assert_eq!(f.sheet["history"], "Od dob prvního majáku.");
    assert_eq!(f.status, Some(Status::Review));
    assert_eq!(f.writer.as_deref(), Some("Petra"));
}

#[test]
fn loads_a_minimal_faction_with_defaults() {
    let g = game(&[
        (
            "factions.toml",
            "[[faction]]\nid = \"cech\"\nname = \"Cech\"\n",
        ),
        ("people/ann.toml", &npc("Ann", "\"cech\"")),
    ]);
    let f = &g.factions[0];
    assert!(f.color.is_none());
    assert!(f.text.is_empty() && f.goals.is_empty() && f.secrets.is_empty());
    assert!(f.sheet.is_empty(), "{:?}", f.sheet);
    assert!(f.status.is_none() && f.writer.is_none());
}

#[test]
fn loads_person_factions_in_file_order() {
    let g = game(&[
        (
            "factions.toml",
            "[[faction]]\nid = \"b-cech\"\nname = \"B\"\n\n[[faction]]\nid = \"a-cech\"\nname = \"A\"\n",
        ),
        ("people/ann.toml", &npc("Ann", "\"b-cech\", \"a-cech\"")),
    ]);
    let factions: Vec<&str> = g.people[&id("ann")]
        .factions
        .iter()
        .map(|f| f.get_ref().as_str())
        .collect();
    assert_eq!(factions, ["b-cech", "a-cech"]);
}

#[test]
fn involves_a_person_named_directly() {
    let g = verona();
    assert!(g.involves(owners_of(&g, "rande"), &id("julie")));
}

#[test]
fn involves_a_member_through_a_faction_owner() {
    let g = verona();
    assert!(g.involves(owners_of(&g, "boj"), &id("romeo")));
    assert!(g.involves(owners_of(&g, "boj"), &id("tybalt")));
}

#[test]
fn does_not_involve_a_non_member_through_a_faction_owner() {
    let g = verona();
    assert!(!g.involves(owners_of(&g, "boj"), &id("julie")));
}

#[test]
fn involves_a_room_named_directly_but_not_through_a_faction() {
    let g = verona();
    assert!(g.involves(owners_of(&g, "pristav-akce"), &id("pristav")));
    assert!(!g.involves(owners_of(&g, "boj"), &id("pristav")));
}

#[test]
fn involves_the_faction_itself_when_named() {
    let g = verona();
    assert!(g.involves(owners_of(&g, "boj"), &id("montekove")));
    assert!(!g.involves(owners_of(&g, "rande"), &id("montekove")));
}

#[test]
fn does_not_involve_anyone_with_no_owners() {
    let g = verona();
    assert!(!g.involves(&[], &id("romeo")));
}

#[test]
fn does_not_involve_an_unknown_id() {
    let g = verona();
    assert!(!g.involves(owners_of(&g, "boj"), &id("nikdo")));
}

#[test]
fn members_lists_every_member_by_id() {
    let g = verona();
    let faction = id("montekove");
    let members: Vec<&str> = g.members(&faction).map(Id::as_str).collect();
    assert_eq!(members, ["romeo", "tybalt"]);
}

#[test]
fn members_of_an_unknown_faction_is_empty() {
    let g = verona();
    assert_eq!(g.members(&id("kapuleti")).count(), 0);
}

#[test]
fn members_of_a_person_id_is_empty() {
    let g = verona();
    assert_eq!(g.members(&id("romeo")).count(), 0);
}

// ---------- loader ----------

#[test]
fn factions_toml_is_not_an_unknown_file() {
    let checked = run(&[("factions.toml", "")]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
}

#[test]
fn unknown_key_in_a_faction_is_an_error() {
    let checked = run(&[(
        "factions.toml",
        "[[faction]]\nid = \"cech\"\nname = \"Cech\"\nmembers = [\"ann\"]\n",
    )]);
    let errors = with(&checked, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].file, "factions.toml");
    assert!(errors[0].message.contains("members"), "{errors:?}");
    assert!(checked.game.is_none());
}

#[test]
fn unknown_top_level_table_in_factions_toml_is_an_error() {
    let checked = run(&[(
        "factions.toml",
        "[[frakce]]\nid = \"cech\"\nname = \"Cech\"\n",
    )]);
    let errors = with(&checked, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].message.contains("frakce"), "{errors:?}");
}

#[test]
fn faction_without_a_name_is_an_error() {
    let checked = run(&[("factions.toml", "[[faction]]\nid = \"cech\"\n")]);
    let errors = with(&checked, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].message.contains("name"), "{errors:?}");
}

#[test]
fn faction_with_an_invalid_id_is_an_error() {
    let checked = run(&[(
        "factions.toml",
        "[[faction]]\nid = \"Montekové\"\nname = \"Montekové\"\n",
    )]);
    assert_eq!(
        checked.count(Severity::Error),
        1,
        "{:?}",
        checked.diagnostics
    );
    assert_eq!(with(&checked, Severity::Error)[0].file, "factions.toml");
}

// ---------- validation: errors ----------

#[test]
fn unknown_faction_in_person_factions_is_an_error() {
    let checked = run(&[
        ("factions.toml", FACTIONS),
        (
            "people/ann.toml",
            &npc("Ann", "\"montekove\", \"kapuleti\""),
        ),
    ]);
    let errors = with(&checked, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].file, "people/ann.toml");
    assert_eq!(
        errors[0].message,
        "unknown faction `kapuleti`, expected a faction"
    );
    assert!(errors[0].span.is_some());
}

#[test]
fn misspelled_faction_suggests_the_closest_one() {
    let checked = run(&[
        ("factions.toml", FACTIONS),
        ("people/ann.toml", &npc("Ann", "\"montekowe\"")),
    ]);
    let errors = with(&checked, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].help.as_deref(), Some("did you mean `montekove`?"));
}

#[test]
fn person_id_in_factions_is_an_error() {
    let checked = run(&[
        ("people/ann.toml", &npc("Ann", "\"bob\"")),
        ("people/bob.toml", &npc("Bob", "")),
    ]);
    assert_eq!(
        messages(&checked, Severity::Error),
        ["`bob` is a person, but a faction must be a faction"]
    );
}

#[test]
fn faction_id_that_clashes_with_a_person_is_an_error() {
    let checked = run(&[
        (
            "factions.toml",
            "[[faction]]\nid = \"ann\"\nname = \"Annini\"\n",
        ),
        ("people/ann.toml", &npc("Ann", "")),
    ]);
    let errors = with(&checked, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].file, "factions.toml");
    assert_eq!(
        errors[0].message,
        "id `ann` is already used by a person in people/ann.toml"
    );
    assert!(errors[0].span.is_some());
}

#[test]
fn faction_id_that_clashes_with_a_room_is_an_error() {
    let checked = run(&[
        (
            "rooms.toml",
            "[[room]]\nid = \"pristav\"\nname = \"Přístav\"\n",
        ),
        (
            "factions.toml",
            "[[faction]]\nid = \"pristav\"\nname = \"Přístavní cech\"\n",
        ),
    ]);
    let errors = messages(&checked, Severity::Error);
    assert_eq!(
        errors,
        ["id `pristav` is already used by a room in rooms.toml"]
    );
}

#[test]
fn faction_id_used_twice_is_an_error() {
    let checked = run(&[
        (
            "factions.toml",
            "[[faction]]\nid = \"cech\"\nname = \"A\"\n\n[[faction]]\nid = \"cech\"\nname = \"B\"\n",
        ),
        ("people/ann.toml", &npc("Ann", "\"cech\"")),
    ]);
    assert_eq!(
        messages(&checked, Severity::Error),
        ["id `cech` is already used by a faction in factions.toml"]
    );
}

#[test]
fn block_id_that_clashes_with_a_faction_is_an_error() {
    let checked = run(&[
        ("factions.toml", FACTIONS),
        ("people/ann.toml", &npc("Ann", "\"montekove\"")),
        (
            "blocks.toml",
            "[[block]]\nid = \"montekove\"\ntext = \"x\"\nshow = [\"ann\"]\n",
        ),
    ]);
    let errors = with(&checked, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].file, "blocks.toml");
    assert_eq!(
        errors[0].message,
        "id `montekove` is already used by a faction in factions.toml"
    );
}

#[test]
fn bad_faction_color_is_an_error() {
    let checked = run(&[
        (
            "factions.toml",
            "[[faction]]\nid = \"cech\"\nname = \"Cech\"\ncolor = \"modrá\"\n",
        ),
        ("people/ann.toml", &npc("Ann", "\"cech\"")),
    ]);
    let errors = with(&checked, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].file, "factions.toml");
    assert_eq!(errors[0].message, "`modrá` is not a color");
    assert!(errors[0].span.is_some());
}

#[test]
fn faction_color_without_a_hash_is_an_error() {
    let checked = run(&[
        (
            "factions.toml",
            "[[faction]]\nid = \"cech\"\nname = \"Cech\"\ncolor = \"2c3e50\"\n",
        ),
        ("people/ann.toml", &npc("Ann", "\"cech\"")),
    ]);
    assert_eq!(
        messages(&checked, Severity::Error),
        ["`2c3e50` is not a color"]
    );
}

// ---------- validation: warnings ----------

#[test]
fn faction_without_members_is_a_warning() {
    let checked = run(&[(
        "factions.toml",
        "[[faction]]\nid = \"cech\"\nname = \"Cech\"\n",
    )]);
    let warnings = with(&checked, Severity::Warning);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(warnings[0].file, "factions.toml");
    assert_eq!(warnings[0].message, "faction `cech` has no members");
    assert!(warnings[0].span.is_some());
    assert!(warnings[0].help.is_some());
    assert!(checked.game.is_some(), "a warning must not stop the game");
}

#[test]
fn faction_with_one_member_gives_no_warning() {
    let checked = run(&[
        (
            "factions.toml",
            "[[faction]]\nid = \"cech\"\nname = \"Cech\"\n",
        ),
        ("people/ann.toml", &npc("Ann", "\"cech\"")),
    ]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
}

#[test]
fn only_the_faction_without_members_gets_the_warning() {
    let checked = run(&[
        (
            "factions.toml",
            "[[faction]]\nid = \"cech\"\nname = \"Cech\"\n\n[[faction]]\nid = \"gilda\"\nname = \"Gilda\"\n",
        ),
        ("people/ann.toml", &npc("Ann", "\"cech\"")),
    ]);
    assert_eq!(
        messages(&checked, Severity::Warning),
        ["faction `gilda` has no members"]
    );
}

// ---------- validation: factions as owners and in `show` ----------

#[test]
fn faction_may_own_an_action() {
    let checked = run(&[
        ("factions.toml", FACTIONS),
        ("people/ann.toml", &npc("Ann", "\"montekove\"")),
        (
            "actions/boj.toml",
            "[action]\ntitle = \"Boj\"\nowners = [\"montekove\"]\n",
        ),
    ]);
    assert!(
        messages(&checked, Severity::Error).is_empty(),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn faction_may_own_a_decision() {
    let checked = run(&[
        ("factions.toml", FACTIONS),
        ("people/ann.toml", &npc("Ann", "\"montekove\"")),
        ("actions/boj.toml", &decision("\"montekove\"")),
    ]);
    assert!(
        messages(&checked, Severity::Error).is_empty(),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn faction_may_own_an_item_and_an_info() {
    let checked = run(&[
        ("factions.toml", FACTIONS),
        ("people/ann.toml", &npc("Ann", "\"montekove\"")),
        (
            "items.toml",
            "[[item]]\nid = \"mec\"\nname = \"Meč\"\nowners = [\"montekove\"]\nstarting = true\nending = true\n",
        ),
        (
            "info.toml",
            "[[info]]\nid = \"heslo\"\nname = \"Heslo\"\nowners = [\"montekove\"]\nstarting = true\nending = true\n",
        ),
    ]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
}

#[test]
fn unknown_owner_still_lists_factions_as_allowed() {
    let checked = run(&[(
        "actions/boj.toml",
        "[action]\ntitle = \"Boj\"\nowners = [\"nikdo\"]\n",
    )]);
    assert_eq!(
        messages(&checked, Severity::Error),
        ["unknown owner `nikdo`, expected a person or a room or a faction"]
    );
}

#[test]
fn block_may_be_shown_to_a_faction() {
    let checked = run(&[
        ("factions.toml", FACTIONS),
        ("people/ann.toml", &npc("Ann", "\"montekove\"")),
        (
            "blocks.toml",
            "[[block]]\nid = \"zvesti\"\ntitle = \"Zvěsti\"\ntext = \"Šeptá se.\"\nshow = [\"montekove\"]\n",
        ),
    ]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
}

#[test]
fn block_shown_to_a_room_is_still_an_error() {
    let checked = run(&[
        (
            "rooms.toml",
            "[[room]]\nid = \"pristav\"\nname = \"Přístav\"\n",
        ),
        (
            "blocks.toml",
            "[[block]]\nid = \"zvesti\"\ntext = \"x\"\nshow = [\"pristav\"]\n",
        ),
    ]);
    assert_eq!(
        messages(&checked, Severity::Error),
        ["`pristav` is a room, but a `show` entry must be a person or a faction"]
    );
}

// ---------- text links ----------

#[test]
fn faction_link_expands_to_the_faction_name() {
    let g = verona();
    assert_eq!(
        larp_model::text::expand_text(&g, "Patříš k [[montekove]]."),
        "Patříš k Montekové."
    );
}

#[test]
fn faction_link_is_not_an_unknown_link() {
    let checked = run(&[
        ("factions.toml", FACTIONS),
        (
            "people/ann.toml",
            "[person]\nname = \"Ann\"\nrole = \"npc\"\nfactions = [\"montekove\"]\nsecrets = [\"Zradíš [[montekove]].\"]\n",
        ),
    ]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
}

#[test]
fn links_inside_faction_texts_are_checked() {
    let checked = run(&[
        (
            "factions.toml",
            "[[faction]]\nid = \"cech\"\nname = \"Cech\"\ngoals = [\"Najít [[nikdo]].\"]\n",
        ),
        ("people/ann.toml", &npc("Ann", "\"cech\"")),
    ]);
    let errors = with(&checked, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].file, "factions.toml");
    assert!(errors[0].message.contains("[[nikdo]]"), "{errors:?}");
}

#[test]
fn expand_replaces_links_in_faction_texts() {
    let g = game(&[
        (
            "factions.toml",
            "[[faction]]\nid = \"cech\"\nname = \"Cech\"\ntext = \"[[ann]]\"\ngoals = [\"Chránit [[ann]].\"]\n\
             secrets = [\"[[ann]] ví.\"]\n\n[faction.sheet]\nhistory = \"Založila [[ann]].\"\n",
        ),
        ("people/ann.toml", &npc("Řehořová Ůlna", "\"cech\"")),
    ]);
    let expanded = larp_model::text::expand(&g);
    let f = &expanded.factions[0];
    assert_eq!(f.text, "Řehořová Ůlna");
    assert_eq!(f.goals, ["Chránit Řehořová Ůlna."]);
    assert_eq!(f.secrets, ["Řehořová Ůlna ví."]);
    assert_eq!(f.sheet["history"], "Založila Řehořová Ůlna.");
}

// ---------- check_people counts faction ownership ----------

#[test]
fn player_owning_only_through_a_faction_gets_no_empty_destiny_warning() {
    let checked = run(&[
        ("factions.toml", FACTIONS),
        ("people/romeo.toml", &player("Romeo", "\"montekove\"")),
        ("actions/boj.toml", &decision("\"montekove\"")),
    ]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
}

#[test]
fn player_with_only_a_faction_action_gets_the_no_decision_warning_not_the_empty_one() {
    let checked = run(&[
        ("factions.toml", FACTIONS),
        ("people/romeo.toml", &player("Romeo", "\"montekove\"")),
        (
            "actions/boj.toml",
            "[action]\ntitle = \"Boj\"\nowners = [\"montekove\"]\n",
        ),
    ]);
    assert_eq!(
        messages(&checked, Severity::Warning),
        ["player has no decision of their own"]
    );
}

#[test]
fn player_outside_the_owning_faction_still_gets_the_empty_destiny_warning() {
    let checked = run(&[
        ("factions.toml", FACTIONS),
        ("people/romeo.toml", &player("Romeo", "\"montekove\"")),
        ("people/julie.toml", &player("Julie", "")),
        ("actions/boj.toml", &decision("\"montekove\"")),
    ]);
    let warnings = with(&checked, Severity::Warning);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(warnings[0].file, "people/julie.toml");
    assert_eq!(
        warnings[0].message,
        "player owns no action or decision; their destiny would be empty"
    );
}

// ---------- lives count faction ownership ----------

const SEGMENTS: &str =
    "[[segment]]\nid = \"s1\"\nname = \"Den\"\n\n[[segment]]\nid = \"s2\"\nname = \"Noc\"\n";

#[test]
fn faction_action_after_a_members_death_is_outside_their_life() {
    let checked = run(&[
        ("segments.toml", SEGMENTS),
        ("factions.toml", FACTIONS),
        (
            "people/tybalt.toml",
            "[person]\nname = \"Tybalt Čapek\"\nrole = \"npc\"\nfactions = [\"montekove\"]\ndies = \"s1\"\n",
        ),
        (
            "actions/pomsta.toml",
            "[action]\ntitle = \"Pomsta\"\nowners = [\"montekove\"]\nsegment = \"s2\"\n",
        ),
    ]);
    let warnings = with(&checked, Severity::Warning);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(warnings[0].file, "actions/pomsta.toml");
    assert!(
        warnings[0]
            .message
            .starts_with("“Tybalt Čapek” acts in “Noc”, after dying"),
        "{warnings:?}"
    );
}

#[test]
fn faction_action_during_a_members_life_is_clean() {
    let checked = run(&[
        ("segments.toml", SEGMENTS),
        ("factions.toml", FACTIONS),
        (
            "people/tybalt.toml",
            "[person]\nname = \"Tybalt Čapek\"\nrole = \"npc\"\nfactions = [\"montekove\"]\ndies = \"s1\"\n",
        ),
        (
            "actions/pomsta.toml",
            "[action]\ntitle = \"Pomsta\"\nowners = [\"montekove\"]\nsegment = \"s1\"\n",
        ),
    ]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
}

#[test]
fn faction_action_after_a_non_members_death_is_not_their_problem() {
    let checked = run(&[
        ("segments.toml", SEGMENTS),
        ("factions.toml", FACTIONS),
        ("people/romeo.toml", &npc("Romeo", "\"montekove\"")),
        (
            "people/julie.toml",
            "[person]\nname = \"Julie\"\nrole = \"npc\"\ndies = \"s1\"\n",
        ),
        (
            "actions/pomsta.toml",
            "[action]\ntitle = \"Pomsta\"\nowners = [\"montekove\"]\nsegment = \"s2\"\n",
        ),
    ]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
}
