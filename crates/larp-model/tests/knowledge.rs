//! Who knows what, and when: `knowledge::knowledge`, `knowledge::known_at_start` and the spoiler
//! check of `[[info]]` links in sheets and briefs.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used, clippy::panic)]

use larp_model::knowledge::{How, Learn, knowledge, known_at_start};
use larp_model::{Checked, Diagnostic, Game, Id, SourceFile, load};

const GAME: &str = "[game]\ntitle = \"Test\"\nlanguage = \"cs\"\n";
const FACTIONS: &str = "[[faction]]\nid = \"mnisi\"\nname = \"Mniši\"\n";
const SEGMENTS: &str = "[[segment]]\nid = \"rano\"\nname = \"Ráno\"\n\n\
    [[segment]]\nid = \"vecer\"\nname = \"Večer\"\n";
/// Bob is an NPC outside any faction.
const BOB: &str = "[person]\nname = \"Bob Řezníček\"\nrole = \"npc\"\n";

/// Ann: an NPC in the `mnisi` faction (so player checks stay quiet), with `extra` lines appended
/// to the `[person]` table (goals, secrets, sub-tables, …).
fn ann(extra: &str) -> String {
    format!("[person]\nname = \"Anna Růžičková\"\nrole = \"npc\"\nfactions = [\"mnisi\"]\n{extra}")
}

/// One `[[info]]`; `owners` is a TOML list body such as `"\"ann\""`.
fn info(id: &str, starting: bool, owners: &str) -> String {
    format!(
        "[[info]]\nid = \"{id}\"\nname = \"Info {id}\"\nstarting = {starting}\nowners = [{owners}]\n\
         ending = {starting}\n\n"
    )
}

/// An action; `segment` may be empty for none. `needs`/`gives` are TOML list bodies.
fn action(
    title: &str,
    owners: &str,
    segment: &str,
    needs: &str,
    gives: &str,
    brief: &str,
) -> String {
    let segment = if segment.is_empty() {
        String::new()
    } else {
        format!("segment = \"{segment}\"\n")
    };
    format!(
        "[action]\ntitle = \"{title}\"\n{segment}owners = [{owners}]\nneeds = [{needs}]\n\
         gives = [{gives}]\nbrief = \"{brief}\"\n"
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

fn id(s: &str) -> Id {
    Id::parse(s).unwrap_or_else(|e| panic!("{s}: {e}"))
}

/// The spoiler warnings only; panics if the game has errors.
fn spoilers(files: &[(&str, &str)]) -> Vec<Diagnostic> {
    let checked = run(files);
    assert!(checked.game.is_some(), "{:?}", checked.diagnostics);
    checked
        .diagnostics
        .into_iter()
        .filter(|d| d.message.contains("names info they don't know yet"))
        .collect()
}

/// The common frame: Ann (with `extra`), Bob, the faction and segments, plus `rest`.
fn with_ann<'a>(extra: &'a str, rest: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
    let mut files = vec![
        ("factions.toml", FACTIONS),
        ("segments.toml", SEGMENTS),
        ("people/bob.toml", BOB),
        ("people/ann.toml", extra),
    ];
    files.extend_from_slice(rest);
    files
}

fn learns_of<'a>(all: &'a [Learn], person: &str) -> Vec<&'a Learn> {
    all.iter().filter(|l| l.person.as_str() == person).collect()
}

fn start(person: &str, info: &str) -> Learn {
    Learn {
        person: id(person),
        info: id(info),
        how: How::Start,
        node: None,
        option: None,
        at: Some(0),
    }
}

// ---- knowledge() ----

#[test]
fn knows_starting_info_from_start_when_owned_directly() {
    let a = ann("");
    let infos = info("heslo", true, "\"bob\"");
    let g = game(&with_ann(&a, &[("info.toml", &infos)]));
    let all = knowledge(&g);
    assert_eq!(all, vec![start("bob", "heslo")]);
}

#[test]
fn knows_starting_info_from_start_when_faction_owns_it() {
    let a = ann("");
    let infos = info("heslo", true, "\"mnisi\"");
    let g = game(&with_ann(&a, &[("info.toml", &infos)]));
    let all = knowledge(&g);
    assert_eq!(all, vec![start("ann", "heslo")]);
}

#[test]
fn does_not_know_from_start_when_info_is_not_starting() {
    let a = ann("");
    let infos = info("heslo", false, "\"ann\"");
    let g = game(&with_ann(&a, &[("info.toml", &infos)]));
    assert!(knowledge(&g).iter().all(|l| l.how != How::Start));
    assert!(known_at_start(&g, &id("ann")).is_empty());
}

#[test]
fn known_at_start_lists_own_and_faction_starting_info_only() {
    let a = ann("");
    let infos = format!(
        "{}{}{}{}",
        info("vlastni", true, "\"ann\""),
        info("frakcni", true, "\"mnisi\""),
        info("cizi", true, "\"bob\""),
        info("pozdejsi", false, "\"ann\""),
    );
    let g = game(&with_ann(&a, &[("info.toml", &infos)]));
    let mut known: Vec<&str> = known_at_start(&g, &id("ann"))
        .into_iter()
        .map(Id::as_str)
        .collect();
    known.sort_unstable();
    assert_eq!(known, vec!["frakcni", "vlastni"]);
    let bob: Vec<&str> = known_at_start(&g, &id("bob"))
        .into_iter()
        .map(Id::as_str)
        .collect();
    assert_eq!(bob, vec!["cizi"]);
}

#[test]
fn finds_info_an_own_action_gives_and_is_told_info_it_needs() {
    let a = ann("");
    let infos = info("tajemstvi", false, "");
    let najit = action("Najít dopis", "\"bob\"", "rano", "", "\"tajemstvi\"", "");
    let rict = action("Říct to", "\"ann\"", "vecer", "\"tajemstvi\"", "", "");
    let g = game(&with_ann(
        &a,
        &[
            ("info.toml", &infos),
            ("actions/najit.toml", &najit),
            ("actions/rict.toml", &rict),
        ],
    ));
    let all = knowledge(&g);
    assert_eq!(
        all,
        vec![
            Learn {
                person: id("ann"),
                info: id("tajemstvi"),
                how: How::Told,
                node: Some(id("rict")),
                option: None,
                at: Some(1),
            },
            Learn {
                person: id("bob"),
                info: id("tajemstvi"),
                how: How::Finds,
                node: Some(id("najit")),
                option: None,
                at: Some(0),
            },
        ]
    );
}

#[test]
fn decision_options_count_with_their_label() {
    let a = ann("");
    let infos = format!("{}{}", info("x", true, "\"bob\""), info("y", false, ""));
    let decision = "[decision]\ntitle = \"Volba\"\nsegment = \"rano\"\nowners = [\"ann\"]\n\n\
        [[decision.option]]\nlabel = \"Přiznat se\"\ngives = [\"y\"]\n\n\
        [[decision.option]]\nlabel = \"Mlčet\"\nneeds = [\"x\"]\n";
    let g = game(&with_ann(
        &a,
        &[("info.toml", &infos), ("actions/volba.toml", decision)],
    ));
    let mine = learns_of(&knowledge(&g), "ann")
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        mine,
        vec![
            Learn {
                person: id("ann"),
                info: id("x"),
                how: How::Told,
                node: Some(id("volba")),
                option: Some("Mlčet".to_owned()),
                at: Some(0),
            },
            Learn {
                person: id("ann"),
                info: id("y"),
                how: How::Finds,
                node: Some(id("volba")),
                option: Some("Přiznat se".to_owned()),
                at: Some(0),
            },
        ]
    );
}

#[test]
fn learns_at_nodes_owned_through_a_faction() {
    let a = ann("");
    let infos = info("y", false, "");
    let act = action("Kapitula", "\"mnisi\"", "rano", "", "\"y\"", "");
    let g = game(&with_ann(
        &a,
        &[("info.toml", &infos), ("actions/kapitula.toml", &act)],
    ));
    let all = knowledge(&g);
    assert_eq!(all.len(), 1, "{all:?}");
    assert_eq!(all[0].person.as_str(), "ann");
    assert_eq!(all[0].how, How::Finds);
    assert_eq!(all[0].node, Some(id("kapitula")));
}

#[test]
fn ignores_items_and_counts_only_info() {
    let a = ann("");
    let items = "[[item]]\nid = \"klic\"\nname = \"Klíč\"\nstarting = true\nowners = [\"ann\"]\nending = true\n\n\
        [[item]]\nid = \"mec\"\nname = \"Meč\"\nending = true\n";
    let infos = info("y", false, "");
    let act = action(
        "Výměna",
        "\"ann\"",
        "rano",
        "\"klic\"",
        "\"mec\", \"y\"",
        "",
    );
    let g = game(&with_ann(
        &a,
        &[
            ("items.toml", items),
            ("info.toml", &infos),
            ("actions/vymena.toml", &act),
        ],
    ));
    let all = knowledge(&g);
    let infos: Vec<&str> = all.iter().map(|l| l.info.as_str()).collect();
    assert_eq!(infos, vec!["y"]);
}

#[test]
fn sorts_by_person_then_info_then_start_timed_untimed() {
    let a = ann("");
    // `zz` comes first in the file, `aa` must still come first in the output.
    let infos = format!(
        "{}{}",
        info("zz", true, "\"ann\", \"bob\""),
        info("aa", false, "")
    );
    let files = with_ann(&a, &[]);
    let pozde = action("Pozdě", "\"ann\"", "vecer", "\"zz\", \"aa\"", "", "");
    let brzy = action("Brzy", "\"ann\"", "rano", "", "\"aa\"", "");
    let kdykoli = action("Kdykoli", "\"ann\", \"bob\"", "", "\"aa\"", "", "");
    let mut files = files;
    files.extend_from_slice(&[
        ("info.toml", infos.as_str()),
        // File names in reverse of the expected order.
        ("actions/a-kdykoli.toml", kdykoli.as_str()),
        ("actions/b-pozde.toml", pozde.as_str()),
        ("actions/c-brzy.toml", brzy.as_str()),
    ]);
    let g = game(&files);
    let got: Vec<String> = knowledge(&g)
        .iter()
        .map(|l| {
            let node = l.node.as_ref().map_or("-", Id::as_str);
            format!("{} {} {:?} {node} {:?}", l.person, l.info, l.how, l.at)
        })
        .collect();
    assert_eq!(
        got,
        vec![
            "ann aa Finds c-brzy Some(0)",
            "ann aa Told b-pozde Some(1)",
            "ann aa Told a-kdykoli None",
            "ann zz Start - Some(0)",
            "ann zz Told b-pozde Some(1)",
            "bob aa Told a-kdykoli None",
            "bob zz Start - Some(0)",
        ]
    );
}

#[test]
fn start_sorts_before_a_node_at_the_same_tick() {
    let a = ann("");
    let infos = info("x", true, "\"ann\"");
    let act = action("Hned", "\"ann\"", "rano", "\"x\"", "", "");
    let g = game(&with_ann(
        &a,
        &[("info.toml", &infos), ("actions/hned.toml", &act)],
    ));
    let hows: Vec<How> = knowledge(&g).iter().map(|l| l.how).collect();
    assert_eq!(hows, vec![How::Start, How::Told]);
}

#[test]
fn at_is_the_first_tick_of_the_node_segment() {
    let a = ann("");
    let segments = "[[segment]]\nid = \"s1\"\nname = \"I\"\n\n[[segment]]\nid = \"s2\"\nname = \"II\"\n\n\
        [[segment]]\nid = \"s3\"\nname = \"III\"\n";
    let infos = info("x", false, "");
    let act = action("Třetí", "\"ann\"", "s3", "", "\"x\"", "");
    let files = vec![
        ("factions.toml", FACTIONS),
        ("segments.toml", segments),
        ("people/ann.toml", a.as_str()),
        ("info.toml", infos.as_str()),
        ("actions/treti.toml", act.as_str()),
    ];
    let g = game(&files);
    let all = knowledge(&g);
    assert_eq!(all.len(), 1, "{all:?}");
    assert_eq!(
        all[0].at,
        larp_model::time::segment(&g, &id("s3")).map(|s| s.start)
    );
    assert_eq!(all[0].at, Some(2));
}

#[test]
fn at_is_none_in_a_game_without_segments() {
    let a = ann("");
    let infos = info("x", false, "");
    let act = action("Bez času", "\"ann\"", "", "", "\"x\"", "");
    let files = vec![
        ("factions.toml", FACTIONS),
        ("people/ann.toml", a.as_str()),
        ("info.toml", infos.as_str()),
        ("actions/bez.toml", act.as_str()),
    ];
    let all = knowledge(&game(&files));
    assert_eq!(all.len(), 1, "{all:?}");
    assert_eq!(all[0].at, None);
}

#[test]
fn removes_duplicate_ways() {
    let a = ann("");
    let infos = info("x", true, "\"bob\"");
    // Owned by Ann directly and through her faction, needing `x` twice.
    let act = action(
        "Dvakrát",
        "\"ann\", \"mnisi\"",
        "rano",
        "\"x\", \"x\"",
        "",
        "",
    );
    let checked = run(&with_ann(
        &a,
        &[("info.toml", &infos), ("actions/dvakrat.toml", &act)],
    ));
    let g = checked
        .game
        .unwrap_or_else(|| panic!("{:?}", checked.diagnostics));
    let mine = learns_of(&knowledge(&g), "ann").len();
    assert_eq!(mine, 1);
}

#[test]
fn how_as_str_names_each_way() {
    assert_eq!(How::Start.as_str(), "start");
    assert_eq!(How::Finds.as_str(), "finds");
    assert_eq!(How::Told.as_str(), "told");
}

// ---- spoiler check: sheets ----

/// `x` is learned by Ann only at a later action, so her sheet must not name it.
fn later_x() -> (String, String) {
    (
        info("x", false, ""),
        action("Výslech u opata", "\"ann\"", "vecer", "", "\"x\"", ""),
    )
}

fn sheet_spoilers(extra: &str, blocks: &str) -> Vec<Diagnostic> {
    let (infos, act) = later_x();
    let a = ann(extra);
    let mut rest = vec![
        ("info.toml", infos.as_str()),
        ("actions/vyslech.toml", act.as_str()),
    ];
    if !blocks.is_empty() {
        rest.push(("blocks.toml", blocks));
    }
    spoilers(&with_ann(&a, &rest))
}

#[test]
fn warns_when_a_goal_names_info_not_known_at_start() {
    let d = sheet_spoilers("goals = [\"Zjistit, že [[x]].\"]\n", "");
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0].file, "people/ann.toml");
    assert_eq!(
        d[0].message,
        "[[x]] in the sheet of `ann` names info they don't know yet"
    );
}

#[test]
fn warns_when_a_secret_names_info_not_known_at_start() {
    let d = sheet_spoilers("secrets = [\"Víš, že [[x]].\"]\n", "");
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0].file, "people/ann.toml");
}

#[test]
fn warns_when_a_sheet_section_names_info_not_known_at_start() {
    let d = sheet_spoilers("\n[person.sheet]\nbackstory = \"Dávno před [[x]].\"\n", "");
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0].file, "people/ann.toml");
}

#[test]
fn warns_when_a_relation_names_info_not_known_at_start() {
    let d = sheet_spoilers(
        "\n[[person.relation]]\nwith = \"bob\"\ntext = \"Řekl ti, že [[x]].\"\n",
        "",
    );
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0].file, "people/ann.toml");
}

#[test]
fn warns_when_a_block_shown_to_the_person_names_info() {
    let blocks = "[[block]]\nid = \"zvesti\"\ntext = \"Šeptá se, že [[x]].\"\nshow = [\"ann\"]\n";
    let d = sheet_spoilers("", blocks);
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0].file, "blocks.toml");
}

#[test]
fn warns_when_a_block_shown_to_their_faction_names_info() {
    let blocks = "[[block]]\nid = \"zvesti\"\ntext = \"Šeptá se, že [[x]].\"\nshow = [\"mnisi\"]\n";
    let d = sheet_spoilers("", blocks);
    assert_eq!(d.len(), 1, "{d:?}");
    assert!(d[0].message.contains("`ann`"), "{d:?}");
}

#[test]
fn does_not_warn_about_a_block_shown_only_to_someone_else() {
    let blocks = "[[block]]\nid = \"zvesti\"\ntext = \"Šeptá se, že [[x]].\"\nshow = [\"bob\"]\n";
    let (infos, act) = later_x();
    let a = ann("");
    // Bob never learns `x`, so he gets the warning; Ann must not.
    let d = spoilers(&with_ann(
        &a,
        &[
            ("info.toml", &infos),
            ("actions/vyslech.toml", &act),
            ("blocks.toml", blocks),
        ],
    ));
    assert_eq!(d.len(), 1, "{d:?}");
    // Reported where the text is written, naming the reader.
    assert_eq!(d[0].file, "blocks.toml");
    assert!(d[0].message.contains("sheet of `bob`"), "{d:?}");
}

#[test]
fn warns_when_info_is_named_through_an_included_block() {
    let blocks = "[[block]]\nid = \"obal\"\ntext = \"Uvnitř: [[x]].\"\n";
    let d = sheet_spoilers("goals = [\"Pamatuj: [[obal]]\"]\n", blocks);
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(
        d[0].message,
        "[[x]] in the sheet of `ann` names info they don't know yet"
    );
}

#[test]
fn warns_when_info_is_named_through_nested_blocks() {
    let blocks = "[[block]]\nid = \"vnejsi\"\ntext = \"A [[vnitrni]].\"\n\n\
        [[block]]\nid = \"vnitrni\"\ntext = \"Uvnitř: [[x]].\"\n";
    let d = sheet_spoilers("goals = [\"Pamatuj: [[vnejsi]]\"]\n", blocks);
    assert_eq!(d.len(), 1, "{d:?}");
}

#[test]
fn does_not_warn_about_starting_info_the_person_owns() {
    let a = ann("goals = [\"Víš, že [[x]] a [[y]].\"]\n");
    let infos = format!(
        "{}{}",
        info("x", true, "\"ann\""),
        info("y", true, "\"mnisi\"")
    );
    let d = spoilers(&with_ann(&a, &[("info.toml", &infos)]));
    assert!(d.is_empty(), "{d:?}");
}

#[test]
fn warns_about_starting_info_owned_by_someone_else() {
    let a = ann("goals = [\"Víš, že [[x]].\"]\n");
    let infos = info("x", true, "\"bob\"");
    let d = spoilers(&with_ann(&a, &[("info.toml", &infos)]));
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0].file, "people/ann.toml");
}

#[test]
fn does_not_warn_about_links_to_non_info() {
    let a = ann("goals = [\"Najdi [[bob]] a [[datum]].\"]\n");
    let blocks = "[[block]]\nid = \"datum\"\ntext = \"13. března\"\n";
    let infos = info("x", true, "\"ann\"");
    let d = spoilers(&with_ann(
        &a,
        &[("info.toml", &infos), ("blocks.toml", blocks)],
    ));
    assert!(d.is_empty(), "{d:?}");
}

#[test]
fn warns_once_per_info_per_sheet() {
    let blocks = "[[block]]\nid = \"zvesti\"\ntext = \"Opět [[x]].\"\nshow = [\"ann\"]\n";
    let d = sheet_spoilers(
        "goals = [\"[[x]] a znovu [[x]]\"]\nsecrets = [\"Taky [[x]].\"]\n",
        blocks,
    );
    assert_eq!(d.len(), 1, "{d:?}");
}

#[test]
fn warns_separately_for_each_info_on_a_sheet() {
    let a = ann("goals = [\"[[x]] a [[y]]\"]\n");
    let infos = format!("{}{}", info("x", false, ""), info("y", false, ""));
    let d = spoilers(&with_ann(&a, &[("info.toml", &infos)]));
    let mut msgs: Vec<&str> = d.iter().map(|d| d.message.as_str()).collect();
    msgs.sort_unstable();
    assert_eq!(
        msgs,
        vec![
            "[[x]] in the sheet of `ann` names info they don't know yet",
            "[[y]] in the sheet of `ann` names info they don't know yet",
        ]
    );
}

// ---- spoiler check: help texts ----

#[test]
fn help_says_finds_it_out_only_at_the_node() {
    let d = sheet_spoilers("goals = [\"[[x]]\"]\n", "");
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(
        d[0].help.as_deref(),
        Some(
            "`ann` finds it out only at “Výslech u opata”; remove the link, or give them the info earlier"
        )
    );
}

#[test]
fn help_says_is_told_only_at_the_node() {
    let a = ann("goals = [\"[[x]]\"]\n");
    let infos = info("x", false, "");
    let dat = action("Dát", "\"bob\"", "rano", "", "\"x\"", "");
    let slyset = action("Slyšet řeč", "\"ann\"", "vecer", "\"x\"", "", "");
    let d = spoilers(&with_ann(
        &a,
        &[
            ("info.toml", &infos),
            ("actions/dat.toml", &dat),
            ("actions/slyset.toml", &slyset),
        ],
    ));
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(
        d[0].help.as_deref(),
        Some("`ann` is told only at “Slyšet řeč”; remove the link, or give them the info earlier")
    );
}

#[test]
fn help_names_the_earliest_node() {
    let a = ann("goals = [\"[[x]]\"]\n");
    let infos = info("x", false, "");
    let pozde = action("Pozdní", "\"ann\"", "vecer", "", "\"x\"", "");
    let brzy = action("Časný", "\"ann\"", "rano", "\"x\"", "", "");
    let dat = action("Dát", "\"bob\"", "rano", "", "\"x\"", "");
    let d = spoilers(&with_ann(
        &a,
        &[
            ("info.toml", &infos),
            ("actions/a-pozde.toml", &pozde),
            ("actions/b-brzy.toml", &brzy),
            ("actions/c-dat.toml", &dat),
        ],
    ));
    assert_eq!(d.len(), 1, "{d:?}");
    assert!(
        d[0].help
            .as_deref()
            .unwrap()
            .contains("is told only at “Časný”"),
        "{d:?}"
    );
}

#[test]
fn help_says_never_learns_it() {
    let a = ann("goals = [\"[[x]]\"]\n");
    let infos = info("x", true, "\"bob\"");
    let d = spoilers(&with_ann(&a, &[("info.toml", &infos)]));
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(
        d[0].help.as_deref(),
        Some(
            "`ann` never learns it; make it `starting` with `ann` among its owners, or remove the link"
        )
    );
}

// ---- spoiler check: briefs ----

fn brief_spoilers(rest: &[(&str, &str)], segments: bool) -> Vec<Diagnostic> {
    let a = ann("");
    let mut files = vec![
        ("factions.toml", FACTIONS),
        ("people/bob.toml", BOB),
        ("people/ann.toml", a.as_str()),
    ];
    if segments {
        files.push(("segments.toml", SEGMENTS));
    }
    files.extend_from_slice(rest);
    spoilers(&files)
}

#[test]
fn brief_may_name_info_learned_at_that_node() {
    let infos = info("x", false, "");
    let act = action("Nález", "\"ann\"", "vecer", "", "\"x\"", "Najdeš [[x]].");
    let d = brief_spoilers(&[("info.toml", &infos), ("actions/nalez.toml", &act)], true);
    assert!(d.is_empty(), "{d:?}");
}

#[test]
fn brief_may_name_info_told_at_that_decision_option() {
    let infos = info("x", false, "");
    let dat = action("Dát", "\"bob\"", "rano", "", "\"x\"", "");
    let decision = "[decision]\ntitle = \"Volba\"\nsegment = \"rano\"\nowners = [\"ann\"]\n\
        brief = \"Víš [[x]].\"\n\n[[decision.option]]\nlabel = \"Ano\"\nneeds = [\"x\"]\n";
    let d = brief_spoilers(
        &[
            ("info.toml", &infos),
            ("actions/dat.toml", &dat),
            ("actions/volba.toml", decision),
        ],
        true,
    );
    assert!(d.is_empty(), "{d:?}");
}

#[test]
fn brief_may_name_info_learned_at_an_earlier_segment() {
    let infos = info("x", false, "");
    let ranni = action("Ranní", "\"ann\"", "rano", "", "\"x\"", "");
    let vecerni = action("Večerní", "\"ann\"", "vecer", "", "", "Pamatuješ [[x]].");
    let d = brief_spoilers(
        &[
            ("info.toml", &infos),
            ("actions/ranni.toml", &ranni),
            ("actions/vecerni.toml", &vecerni),
        ],
        true,
    );
    assert!(d.is_empty(), "{d:?}");
}

#[test]
fn brief_may_name_info_known_from_the_start() {
    let infos = info("x", true, "\"mnisi\"");
    let act = action("Ranní", "\"ann\"", "rano", "", "", "Víš [[x]].");
    let d = brief_spoilers(&[("info.toml", &infos), ("actions/ranni.toml", &act)], true);
    assert!(d.is_empty(), "{d:?}");
}

#[test]
fn warns_when_brief_names_info_learned_only_at_a_later_segment() {
    let infos = info("x", false, "");
    let ranni = action("Ranní řeč", "\"ann\"", "rano", "", "", "Víš [[x]].");
    let vecerni = action("Večerní nález", "\"ann\"", "vecer", "", "\"x\"", "");
    let d = brief_spoilers(
        &[
            ("info.toml", &infos),
            ("actions/ranni.toml", &ranni),
            ("actions/vecerni.toml", &vecerni),
        ],
        true,
    );
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0].file, "actions/ranni.toml");
    assert_eq!(
        d[0].message,
        "[[x]] in the brief of “Ranní řeč” of `ann` names info they don't know yet"
    );
    assert_eq!(
        d[0].help.as_deref(),
        Some(
            "`ann` finds it out only at “Večerní nález”; remove the link, or give them the info earlier"
        )
    );
}

#[test]
fn warns_when_brief_names_info_the_owner_never_learns() {
    let infos = info("x", true, "\"bob\"");
    let act = action("Ranní", "\"ann\"", "rano", "", "", "Víš [[x]].");
    let d = brief_spoilers(&[("info.toml", &infos), ("actions/ranni.toml", &act)], true);
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0].file, "actions/ranni.toml");
    assert!(
        d[0].help.as_deref().unwrap().contains("never learns it"),
        "{d:?}"
    );
}

#[test]
fn warns_about_a_brief_through_an_included_block() {
    let infos = info("x", false, "");
    let blocks = "[[block]]\nid = \"obal\"\ntext = \"Uvnitř [[x]].\"\n";
    let ranni = action("Ranní", "\"ann\"", "rano", "", "", "Víš [[obal]].");
    let vecerni = action("Večerní", "\"ann\"", "vecer", "", "\"x\"", "");
    let d = brief_spoilers(
        &[
            ("info.toml", &infos),
            ("blocks.toml", blocks),
            ("actions/ranni.toml", &ranni),
            ("actions/vecerni.toml", &vecerni),
        ],
        true,
    );
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0].file, "actions/ranni.toml");
}

#[test]
fn warns_each_owner_of_a_brief_who_does_not_know_the_info() {
    let infos = info("x", true, "\"ann\"");
    let act = action("Společná", "\"ann\", \"bob\"", "rano", "", "", "Víš [[x]].");
    let d = brief_spoilers(
        &[("info.toml", &infos), ("actions/spolecna.toml", &act)],
        true,
    );
    assert_eq!(d.len(), 1, "{d:?}");
    assert!(d[0].message.contains("`bob`"), "{d:?}");
}

#[test]
fn without_segments_any_node_where_they_learn_it_counts() {
    let infos = info("x", false, "");
    let prvni = action("První", "\"ann\"", "", "", "", "Víš [[x]].");
    let druha = action("Druhá", "\"ann\"", "", "", "\"x\"", "");
    let d = brief_spoilers(
        &[
            ("info.toml", &infos),
            ("actions/prvni.toml", &prvni),
            ("actions/druha.toml", &druha),
        ],
        false,
    );
    assert!(d.is_empty(), "{d:?}");
}

#[test]
fn without_segments_the_sheet_is_still_checked() {
    let a = ann("goals = [\"[[x]]\"]\n");
    let infos = info("x", false, "");
    let druha = action("Druhá", "\"ann\"", "", "", "\"x\"", "");
    let files = vec![
        ("factions.toml", FACTIONS),
        ("people/ann.toml", a.as_str()),
        ("info.toml", infos.as_str()),
        ("actions/druha.toml", druha.as_str()),
    ];
    let d = spoilers(&files);
    assert_eq!(d.len(), 1, "{d:?}");
}

#[test]
fn help_names_the_node_of_the_warned_info_not_of_another() {
    let a = ann("goals = [\"[[x]]\"]\n");
    // `a-jine` sorts before `x`, and Ann learns it earlier; the help must still name `x`'s node.
    let infos = format!("{}{}", info("a-jine", false, ""), info("x", false, ""));
    let jine = action("Jiná věc", "\"ann\"", "rano", "", "\"a-jine\"", "");
    let vyslech = action("Výslech", "\"ann\"", "vecer", "", "\"x\"", "");
    let d = spoilers(&with_ann(
        &a,
        &[
            ("info.toml", &infos),
            ("actions/jine.toml", &jine),
            ("actions/vyslech.toml", &vyslech),
        ],
    ));
    assert_eq!(d.len(), 1, "{d:?}");
    assert!(
        d[0].help.as_deref().unwrap().contains("only at “Výslech”"),
        "{d:?}"
    );
}

#[test]
fn faction_texts_on_a_members_sheet_are_checked_too() {
    let factions = "[[faction]]\nid = \"mnisi\"\nname = \"Mniši\"\n\
        secrets = [\"Opat zná [[tajne]].\"]\n";
    let ann = ann("");
    let infos = info("tajne", false, "");
    let found = spoilers(&[
        ("factions.toml", factions),
        ("people/ann.toml", &ann),
        ("people/bob.toml", BOB),
        ("info.toml", &infos),
    ]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].file, "factions.toml");
    assert!(
        found[0]
            .message
            .starts_with("[[tajne]] in the sheet of `ann`"),
        "{found:?}"
    );
}

#[test]
fn organizers_are_not_checked() {
    let org = "[person]\nname = \"Olga\"\nrole = \"organizer\"\ngoals = [\"Hlídej [[x]].\"]\n";
    let (infos, act) = later_x();
    let a = ann("");
    let d = spoilers(&with_ann(
        &a,
        &[
            ("info.toml", &infos),
            ("actions/vyslech.toml", &act),
            ("people/olga.toml", org),
        ],
    ));
    assert!(d.iter().all(|w| !w.message.contains("`olga`")), "{d:?}");
}

#[test]
fn info_learned_in_the_same_segment_does_not_count_as_earlier() {
    let infos = info("x", false, "");
    let find = action("Nález", "\"ann\"", "vecer", "", "\"x\"", "");
    let tell = action("Řeč", "\"ann\"", "vecer", "", "", "Víš [[x]].");
    let a = ann("");
    let d = spoilers(&with_ann(
        &a,
        &[
            ("info.toml", &infos),
            ("actions/nalez.toml", &find),
            ("actions/rec.toml", &tell),
        ],
    ));
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0].file, "actions/rec.toml");
}

#[test]
fn info_learned_only_after_an_option_is_not_known_in_later_briefs() {
    let infos = info("x", false, "");
    let decision = "[decision]\ntitle = \"Volba\"\nsegment = \"rano\"\nowners = [\"ann\"]\n\n\
        [[decision.option]]\nlabel = \"Ano\"\ngives = [\"x\"]\n\n\
        [[decision.option]]\nlabel = \"Ne\"\n";
    let later = action("Večerní řeč", "\"ann\"", "vecer", "", "", "Víš [[x]].");
    let a = ann("");
    let d = spoilers(&with_ann(
        &a,
        &[
            ("info.toml", &infos),
            ("actions/volba.toml", decision),
            ("actions/rec.toml", &later),
        ],
    ));
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(
        d[0].help.as_deref(),
        Some(
            "`ann` finds it out only at “Volba” after choosing “Ano”; remove the link, or give them the info earlier"
        )
    );
}

#[test]
fn a_node_without_time_in_a_game_with_segments_does_not_count() {
    let infos = info("x", false, "");
    let find = action("Nález", "\"ann\"", "", "", "\"x\"", "");
    let tell = action("Řeč", "\"ann\"", "vecer", "", "", "Víš [[x]].");
    let a = ann("");
    let d = spoilers(&with_ann(
        &a,
        &[
            ("info.toml", &infos),
            ("actions/nalez.toml", &find),
            ("actions/rec.toml", &tell),
        ],
    ));
    assert_eq!(d.len(), 1, "{d:?}");
}
