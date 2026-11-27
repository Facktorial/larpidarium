//! Behavior of `larp_model::load` on small in-memory games.

use larp_model::{Checked, Severity, SourceFile, load};

const GAME: &str = "[game]\ntitle = \"Test\"\nlanguage = \"en\"\n";
const PERSON: &str = "[person]\nname = \"Ann\"\nrole = \"player\"\n";

fn run(files: &[(&str, &str)]) -> Checked {
    let files: Vec<SourceFile> = files
        .iter()
        .map(|(path, text)| SourceFile {
            path: (*path).to_owned(),
            text: (*text).to_owned(),
        })
        .collect();
    load(&files)
}

/// Messages of all diagnostics with the given severity.
fn messages(checked: &Checked, severity: Severity) -> Vec<&str> {
    checked
        .diagnostics
        .iter()
        .filter(|d| d.severity == severity)
        .map(|d| d.message.as_str())
        .collect()
}

#[test]
fn minimal_game_is_valid() {
    let checked = run(&[("game.toml", GAME)]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    assert_eq!(checked.game.unwrap().meta.title, "Test");
}

#[test]
fn missing_game_toml_is_an_error() {
    let checked = run(&[]);
    assert!(checked.game.is_none());
    assert_eq!(
        messages(&checked, Severity::Error),
        ["game.toml is missing"]
    );
}

#[test]
fn toml_syntax_error_has_a_span() {
    let checked = run(&[("game.toml", "[game\n")]);
    let d = &checked.diagnostics[0];
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(d.file, "game.toml");
    assert!(d.span.is_some());
}

#[test]
fn unknown_key_is_an_error() {
    let checked = run(&[("game.toml", &format!("{GAME}subtitle = \"x\"\n"))]);
    assert!(checked.game.is_none());
    assert!(messages(&checked, Severity::Error)[0].contains("subtitle"));
}

#[test]
fn file_name_must_be_a_valid_id() {
    let checked = run(&[
        ("game.toml", GAME),
        ("actions/Bad Name.toml", "[action]\ntitle = \"x\"\n"),
    ]);
    assert!(messages(&checked, Severity::Error)[0].contains("file name is not a valid id"));
}

#[test]
fn action_file_needs_exactly_one_table() {
    let both = "[action]\ntitle = \"a\"\n[decision]\ntitle = \"d\"\n";
    let checked = run(&[("game.toml", GAME), ("actions/x.toml", both)]);
    assert!(messages(&checked, Severity::Error)[0].contains("both"));

    let checked = run(&[("game.toml", GAME), ("actions/x.toml", "")]);
    assert!(messages(&checked, Severity::Error)[0].contains("neither"));
}

#[test]
fn unknown_reference_suggests_the_closest_id() {
    let items = "[[item]]\nid = \"rusty-key\"\nname = \"Key\"\nstarting = true\n";
    let action = "[action]\ntitle = \"Open\"\nneeds = [\"rusty-kee\"]\n";
    let checked = run(&[
        ("game.toml", GAME),
        ("items.toml", items),
        ("actions/open.toml", action),
    ]);
    let error = checked
        .diagnostics
        .iter()
        .find(|d| d.severity == Severity::Error)
        .unwrap();
    assert_eq!(error.file, "actions/open.toml");
    assert_eq!(error.help.as_deref(), Some("did you mean `rusty-key`?"));
    assert_eq!(&action[error.span.clone().unwrap()], "\"rusty-kee\"");
}

#[test]
fn reference_of_the_wrong_kind_is_an_error() {
    let action = "[action]\ntitle = \"x\"\nneeds = [\"ann\"]\n";
    let checked = run(&[
        ("game.toml", GAME),
        ("people/ann.toml", PERSON),
        ("actions/x.toml", action),
    ]);
    assert_eq!(
        messages(&checked, Severity::Error),
        ["`ann` is a person, but a need must be an item or an info"]
    );
}

#[test]
fn duplicate_ids_are_an_error() {
    let rooms = "[[room]]\nid = \"ann\"\nname = \"Ann's room\"\n";
    let checked = run(&[
        ("game.toml", GAME),
        ("people/ann.toml", PERSON),
        ("rooms.toml", rooms),
    ]);
    assert!(messages(&checked, Severity::Error)[0].contains("already used by a person"));
}

#[test]
fn needed_but_never_given_is_an_error() {
    let info = "[[info]]\nid = \"secret\"\nname = \"Secret\"\n";
    let action = "[action]\ntitle = \"x\"\nneeds = [\"secret\"]\n";
    let checked = run(&[
        ("game.toml", GAME),
        ("info.toml", info),
        ("actions/x.toml", action),
    ]);
    assert_eq!(
        messages(&checked, Severity::Error),
        ["`secret` is needed here, but nothing gives it"]
    );
}

#[test]
fn given_but_never_needed_is_a_warning_unless_ending() {
    let action = "[action]\ntitle = \"x\"\ngives = [\"secret\"]\n";
    let info = "[[info]]\nid = \"secret\"\nname = \"Secret\"\n";
    let checked = run(&[
        ("game.toml", GAME),
        ("info.toml", info),
        ("actions/x.toml", action),
    ]);
    assert!(checked.game.is_some(), "warnings must not block the game");
    assert_eq!(
        messages(&checked, Severity::Warning),
        ["`secret` is never needed by any action"]
    );

    let ending = format!("{info}ending = true\n");
    let checked = run(&[
        ("game.toml", GAME),
        ("info.toml", &ending),
        ("actions/x.toml", action),
    ]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
}

#[test]
fn loop_of_actions_is_a_warning() {
    let info = "[[info]]\nid = \"egg\"\nname = \"Egg\"\n\n[[info]]\nid = \"hen\"\nname = \"Hen\"\n";
    let egg = "[action]\ntitle = \"Lay\"\nneeds = [\"hen\"]\ngives = [\"egg\"]\n";
    let hen = "[action]\ntitle = \"Hatch\"\nneeds = [\"egg\"]\ngives = [\"hen\"]\n";
    let checked = run(&[
        ("game.toml", GAME),
        ("info.toml", info),
        ("actions/lay.toml", egg),
        ("actions/hatch.toml", hen),
    ]);
    let warnings = messages(&checked, Severity::Warning);
    assert_eq!(warnings.len(), 2, "{warnings:?}");
    assert!(warnings.iter().all(|w| w.contains("can never happen")));
}

#[test]
fn invalid_color_is_an_error() {
    let person = format!("{PERSON}color = \"blue\"\n");
    let checked = run(&[("game.toml", GAME), ("people/ann.toml", &person)]);
    assert_eq!(
        messages(&checked, Severity::Error),
        ["`blue` is not a color"]
    );
}

#[test]
fn czech_text_is_accepted() {
    let game = "[game]\ntitle = \"Ztracené opatství\"\nlanguage = \"cs\"\n";
    let person = "[person]\nname = \"Bratr Anselm\"\nrole = \"npc\"\n";
    let checked = run(&[("game.toml", game), ("people/brother-anselm.toml", person)]);
    let game = checked.game.unwrap();
    assert_eq!(game.meta.title, "Ztracené opatství");
}

#[test]
fn unknown_file_is_a_warning() {
    let checked = run(&[("game.toml", GAME), ("notes.toml", "")]);
    assert!(checked.game.is_some());
    assert_eq!(
        messages(&checked, Severity::Warning),
        ["unknown file, ignored"]
    );
}

// ---------- level 3: playthroughs ----------

const KEY: &str = "[[item]]\nid = \"key\"\nname = \"Key\"\nstarting = true\n";

fn ending(id: &str) -> String {
    format!("[[info]]\nid = \"{id}\"\nname = \"{id}\"\nending = true\n")
}

#[test]
fn info_cannot_be_used_up() {
    let info = "[[info]]\nid = \"rumor\"\nname = \"Rumor\"\nstarting = true\n";
    let action = "[action]\ntitle = \"x\"\nuses = [\"rumor\"]\n";
    let checked = run(&[
        ("game.toml", GAME),
        ("info.toml", info),
        ("actions/x.toml", action),
    ]);
    assert_eq!(
        messages(&checked, Severity::Error),
        ["`rumor` is an info, but a use must be an item"]
    );
}

#[test]
fn simulation_finds_every_ending_with_a_route() {
    let decision = "[decision]\ntitle = \"Door\"\n\n[[decision.option]]\nlabel = \"Open\"\nneeds = [\"key\"]\ngives = [\"win\"]\n\n[[decision.option]]\nlabel = \"Flee\"\ngives = [\"lose\"]\n";
    let info = format!("{}\n{}", ending("win"), ending("lose"));
    let checked = run(&[
        ("game.toml", GAME),
        ("items.toml", KEY),
        ("info.toml", &info),
        ("actions/door.toml", decision),
    ]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    let p = checked.playthroughs.unwrap();
    assert_eq!(p.endings.len(), 2);
    let win = p
        .endings
        .iter()
        .find(|(id, _)| id.as_str() == "win")
        .unwrap()
        .1;
    assert_eq!(win, &["Door → Open"]);
}

#[test]
fn used_up_item_makes_competing_choices_exclusive() {
    // Both actions use the single key: each playthrough can reach only one ending.
    let a = "[action]\ntitle = \"Open chest\"\nuses = [\"key\"]\ngives = [\"gold\"]\n";
    let b = "[action]\ntitle = \"Open cell\"\nuses = [\"key\"]\ngives = [\"freedom\"]\n";
    let info = format!("{}\n{}", ending("gold"), ending("freedom"));
    let checked = run(&[
        ("game.toml", GAME),
        ("items.toml", KEY),
        ("info.toml", &info),
        ("actions/chest.toml", a),
        ("actions/cell.toml", b),
    ]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    let p = checked.playthroughs.unwrap();
    assert_eq!(
        p.endings.len(),
        2,
        "both endings reachable, on different paths"
    );
    assert_eq!(p.end_states, 2);
}

#[test]
fn needs_keeps_the_item_for_later() {
    let a = "[action]\ntitle = \"Look\"\nneeds = [\"key\"]\ngives = [\"seen\"]\n";
    let b = "[action]\ntitle = \"Open\"\nneeds = [\"key\", \"seen\"]\ngives = [\"win\"]\n";
    let info = format!(
        "[[info]]\nid = \"seen\"\nname = \"Seen\"\n\n{}",
        ending("win")
    );
    let checked = run(&[
        ("game.toml", GAME),
        ("items.toml", KEY),
        ("info.toml", &info),
        ("actions/look.toml", a),
        ("actions/open.toml", b),
    ]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
}

#[test]
fn unreachable_ending_is_an_error() {
    // Burning uses the key up before the door decision can need it.
    let burn = "[action]\ntitle = \"Melt key\"\nuses = [\"key\"]\ngives = [\"slag\"]\n";
    let door = "[action]\ntitle = \"Open door\"\nneeds = [\"key\", \"slag\"]\ngives = [\"win\"]\n";
    let info = format!(
        "[[info]]\nid = \"slag\"\nname = \"Slag\"\n\n{}",
        ending("win")
    );
    let checked = run(&[
        ("game.toml", GAME),
        ("items.toml", KEY),
        ("info.toml", &info),
        ("actions/burn.toml", burn),
        ("actions/door.toml", door),
    ]);
    let errors = messages(&checked, Severity::Error);
    assert_eq!(
        errors,
        ["ending `win` cannot be reached in any playthrough"]
    );
    assert!(
        messages(&checked, Severity::Warning)
            .iter()
            .any(|w| w.contains("“Open door” can never happen"))
    );
}

#[test]
fn stuck_playthrough_is_a_warning() {
    let decision = "[decision]\ntitle = \"Path\"\n\n[[decision.option]]\nlabel = \"Left\"\ngives = [\"win\"]\n\n[[decision.option]]\nlabel = \"Right\"\n";
    let checked = run(&[
        ("game.toml", GAME),
        ("info.toml", &ending("win")),
        ("actions/path.toml", decision),
    ]);
    let warnings = messages(&checked, Severity::Warning);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("gets stuck"));
    assert!(warnings[0].contains("Path → Right"));
}

#[test]
fn option_that_can_never_be_chosen_is_a_warning() {
    let decision = "[decision]\ntitle = \"Door\"\n\n[[decision.option]]\nlabel = \"Open\"\nneeds = [\"never\"]\n\n[[decision.option]]\nlabel = \"Knock\"\n";
    let info = "[[info]]\nid = \"never\"\nname = \"Never\"\n";
    let other = "[action]\ntitle = \"Make\"\nneeds = [\"never\"]\ngives = [\"never\"]\n";
    let checked = run(&[
        ("game.toml", GAME),
        ("info.toml", info),
        ("actions/door.toml", decision),
        ("actions/make.toml", other),
    ]);
    let warnings = messages(&checked, Severity::Warning);
    assert!(
        warnings.contains(&"option “Door → Open” can never be chosen"),
        "{warnings:?}"
    );
}

// ---------- levels 4 and 5: people and sheets ----------

const FULL_PLAYER: &str = "[person]\nname = \"Ann\"\nrole = \"player\"\ngoals = [\"Win\"]\n\n[person.sheet]\nbackstory = \"b\"\nwhereabouts = \"w\"\n";

#[test]
fn player_without_actions_or_sheet_text_gets_warnings() {
    let checked = run(&[("game.toml", GAME), ("people/ann.toml", PERSON)]);
    let warnings = messages(&checked, Severity::Warning);
    assert!(
        warnings.iter().any(|w| w.contains("owns no action")),
        "{warnings:?}"
    );
    assert!(warnings.iter().any(|w| w.contains("`backstory`")));
    assert!(warnings.iter().any(|w| w.contains("`whereabouts`")));
    assert!(warnings.iter().any(|w| w.contains("no goals")));
}

#[test]
fn player_with_actions_but_no_decision_gets_a_warning() {
    let action = "[action]\ntitle = \"x\"\nowners = [\"ann\"]\n";
    let checked = run(&[
        ("game.toml", GAME),
        ("people/ann.toml", FULL_PLAYER),
        ("actions/x.toml", action),
    ]);
    assert_eq!(
        messages(&checked, Severity::Warning),
        ["player has no decision of their own"]
    );
}

#[test]
fn complete_player_with_a_decision_is_clean() {
    let decision =
        "[decision]\ntitle = \"d\"\nowners = [\"ann\"]\n\n[[decision.option]]\nlabel = \"a\"\n";
    let checked = run(&[
        ("game.toml", GAME),
        ("people/ann.toml", FULL_PLAYER),
        ("actions/d.toml", decision),
    ]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
}

#[test]
fn relation_to_unknown_person_is_an_error() {
    let person = format!("{FULL_PLAYER}\n[[person.relation]]\nwith = \"bob\"\ntext = \"?\"\n");
    let checked = run(&[("game.toml", GAME), ("people/ann.toml", &person)]);
    assert_eq!(
        messages(&checked, Severity::Error),
        ["unknown relation `bob`, expected a person"]
    );
}

// ---------- segments ----------

const SEGMENTS: &str = "[[segment]]\nid = \"act-one\"\nname = \"Act I\"\ncolor = \"#336699\"\n";

#[test]
fn nodes_can_belong_to_segments() {
    let checked = run(&[
        ("game.toml", GAME),
        ("segments.toml", SEGMENTS),
        (
            "actions/a.toml",
            "[action]\ntitle = \"A\"\nsegment = \"act-one\"\n",
        ),
    ]);
    assert_eq!(messages(&checked, Severity::Error), Vec::<&str>::new());
    let game = checked.game.unwrap();
    assert_eq!(game.segments[0].name, "Act I");
    assert_eq!(
        game.nodes
            .values()
            .next()
            .unwrap()
            .segment()
            .unwrap()
            .get_ref()
            .as_str(),
        "act-one"
    );
}

#[test]
fn unknown_segment_and_bad_color_are_errors() {
    let checked = run(&[
        ("game.toml", GAME),
        (
            "segments.toml",
            "[[segment]]\nid = \"act-one\"\nname = \"Act I\"\ncolor = \"blue\"\n",
        ),
        (
            "actions/a.toml",
            "[action]\ntitle = \"A\"\nsegment = \"act-two\"\n",
        ),
    ]);
    let errors = messages(&checked, Severity::Error);
    assert!(errors.iter().any(|m| m.contains("act-two")), "{errors:?}");
    assert!(errors.iter().any(|m| m.contains("blue")), "{errors:?}");
}

#[test]
fn games_with_errors_are_kept_as_draft() {
    let checked = run(&[
        ("game.toml", GAME),
        (
            "actions/a.toml",
            "[action]\ntitle = \"A\"\nneeds = [\"nothing\"]\n",
        ),
    ]);
    assert!(checked.game.is_none());
    let draft = checked.draft.unwrap();
    assert_eq!(draft.nodes.len(), 1);
}

#[test]
fn valid_games_have_no_draft() {
    let checked = run(&[("game.toml", GAME)]);
    assert!(checked.game.is_some());
    assert!(checked.draft.is_none());
}
