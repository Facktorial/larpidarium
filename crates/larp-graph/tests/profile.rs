//! A character's profile: fields, what others think of them, what they start with, and the
//! timeline of their life, phases and actions in world-time order.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used)]

use larp_graph::{Moment, Profile, profile};
use larp_model::{Game, Id, SourceFile, load};

/// Two steps and two world segments, so a segment is 2 ticks long. Ann's cycle covers only the
/// first segment, so its steps are 1 tick each: step 1 = tick 0, step 2 = tick 1.
const GAME: &str = "[game]\ntitle = \"Profil\"\nlanguage = \"cs\"\nsteps = [\"Ty\", \"Potřeba\"]\n";
const SEGMENTS: &str =
    "[[segment]]\nid = \"s1\"\nname = \"Noc ř\"\n\n[[segment]]\nid = \"s2\"\nname = \"Ráno ů\"\n";
const CYCLES: &str =
    "[[cycle]]\nid = \"c-ann\"\nname = \"Annin cyklus\"\nhero = \"ann\"\nfrom = \"s1\"\n";
const PLAYERS: &str = "[[player]]\nid = \"p1\"\nname = \"Petra Ůlehlová\"\n";
const FACTIONS: &str = "[[faction]]\nid = \"rad\"\nname = \"Řád\"\n";
const ROOMS: &str = "[[room]]\nid = \"kaple\"\nname = \"Kaple\"\n";
const ITEMS: &str = "[[item]]\nid = \"klic\"\nname = \"Klíč\"\nowners = [\"ann\"]\nstarting = true\n\n\
    [[item]]\nid = \"mec\"\nname = \"Meč\"\nowners = [\"rad\"]\nstarting = true\n\n\
    [[item]]\nid = \"lampa\"\nname = \"Lampa\"\nowners = [\"bob\"]\nstarting = true\n\n\
    [[item]]\nid = \"svice\"\nname = \"Svíce\"\n";
const INFO: &str = "[[info]]\nid = \"heslo\"\nname = \"Heslo\"\nowners = [\"rad\"]\nstarting = true\n\n\
    [[info]]\nid = \"tip\"\nname = \"Tip\"\n\n[[info]]\nid = \"zprava\"\nname = \"Zpráva\"\n\n\
    [[info]]\nid = \"konec\"\nname = \"Konec\"\nending = true\n";

const ANN: &str = "[person]\nname = \"Anna Řehořová\"\nrole = \"player\"\ncolor = \"#c0392b\"\n\
    played_by = \"p1\"\nfactions = [\"rad\"]\ndies = 2\nstatus = \"review\"\nwriter = \"Petra\"\n\
    goals = [\"Přežít\", \"Najít klíč\"]\nsecrets = [\"Lže\"]\n\n\
    [person.sheet]\nwhereabouts = \"V kapli\"\nappearance = \"Černý plášť\"\n\n\
    [[person.relation]]\nwith = \"bob\"\ntext = \"Bratr, kterému nevěří\"\n\n\
    [[person.phase]]\nstep = 2\ngoals = [\"Utéct\", \"Skrýt klíč\"]\nsecrets = [\"Zradila\", \"Má dluh\"]\n";
const BOB: &str = "[person]\nname = \"Bob Čech\"\nrole = \"player\"\n\n\
    [[person.relation]]\nwith = \"ann\"\ntext = \"Bojí se jí\"\n\n\
    [[person.relation]]\nwith = \"cyril\"\ntext = \"Neví o něm\"\n";
const CYRIL: &str = "[person]\nname = \"Cyril\"\nrole = \"npc\"\nenters = \"s2\"\ndies = \"s2\"\n\n\
    [[person.relation]]\nwith = \"ann\"\ntext = \"Dluží jí\"\n\n\
    [[person.phase]]\nsegment = \"s2\"\ngoals = [\"Hlídat\"]\n";

const ACTIONS: [(&str, &str); 6] = [
    (
        "actions/a-hledani.toml",
        "[action]\ntitle = \"Hledání\"\ncycle = \"c-ann\"\nstep = 1\nowners = [\"ann\", \"bob\", \"kaple\", \"rad\"]\n\
         needs = [\"klic\", \"heslo\"]\ngives = [\"tip\"]\n",
    ),
    (
        "actions/b-rad.toml",
        "[action]\ntitle = \"Porada řádu\"\nsegment = \"s2\"\nowners = [\"rad\"]\nneeds = [\"tip\"]\n\
         gives = [\"svice\", \"zprava\"]\nuses = [\"mec\"]\n",
    ),
    (
        "actions/c-volba.toml",
        "[decision]\ntitle = \"Volba\"\ncycle = \"c-ann\"\nstep = 2\nowners = [\"ann\"]\n\
         brief = \"Co uděláš?\"\ntext = \"Org: čekej.\"\n\n\
         [[decision.option]]\nlabel = \"Zůstat\"\nneeds = [\"zprava\"]\nuses = [\"svice\"]\ngives = [\"konec\"]\n\n\
         [[decision.option]]\nlabel = \"Odejít\"\nuses = [\"klic\"]\n\n\
         [[decision.option]]\nlabel = \"Utéct\"\nuses = [\"svice\"]\n",
    ),
    (
        "actions/d-bob.toml",
        "[action]\ntitle = \"Bobova věc\"\nsegment = \"s1\"\nowners = [\"bob\"]\nneeds = [\"lampa\"]\n",
    ),
    (
        "actions/e-volno.toml",
        "[action]\ntitle = \"Volno\"\nowners = [\"ann\"]\n",
    ),
    (
        "actions/f-straz.toml",
        "[action]\ntitle = \"Stráž\"\nsegment = \"s2\"\nowners = [\"cyril\"]\n",
    ),
];

/// An empty list of names, for `assert_eq!`.
const NONE: [&str; 0] = [];

fn game(files: &[(&str, &str)]) -> Game {
    let files: Vec<SourceFile> = files
        .iter()
        .map(|(path, text)| SourceFile {
            path: (*path).to_owned(),
            text: (*text).to_owned(),
        })
        .collect();
    let checked = load(&files);
    assert!(checked.game.is_some(), "{:#?}", checked.diagnostics);
    checked.game.unwrap()
}

fn fixture() -> Game {
    fixture_with(&[])
}

/// [`fixture`] with more files.
fn fixture_with(extra: &[(&str, &str)]) -> Game {
    let mut files = vec![
        ("game.toml", GAME),
        ("segments.toml", SEGMENTS),
        ("cycles.toml", CYCLES),
        ("players.toml", PLAYERS),
        ("factions.toml", FACTIONS),
        ("rooms.toml", ROOMS),
        ("items.toml", ITEMS),
        ("info.toml", INFO),
        ("people/ann.toml", ANN),
        ("people/bob.toml", BOB),
        ("people/cyril.toml", CYRIL),
    ];
    files.extend(ACTIONS);
    files.extend_from_slice(extra);
    game(&files)
}

fn id(s: &str) -> Id {
    Id::parse(s).unwrap()
}

fn profile_of(g: &Game, person: &str) -> Profile {
    profile(g, &id(person)).unwrap()
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
        .collect()
}

fn moment<'a>(p: &'a Profile, node: &str) -> &'a Moment {
    p.timeline
        .iter()
        .find(|m| m.node.as_deref() == Some(node))
        .unwrap()
}

fn life<'a>(p: &'a Profile, kind: &str) -> &'a Moment {
    p.timeline.iter().find(|m| m.kind == kind).unwrap()
}

// ---------- basic fields ----------

#[test]
fn returns_none_when_the_id_is_not_a_person() {
    let g = fixture();
    assert!(profile(&g, &id("nobody")).is_none());
    // Rooms, factions and items are not people either.
    assert!(profile(&g, &id("kaple")).is_none());
    assert!(profile(&g, &id("rad")).is_none());
    assert!(profile(&g, &id("klic")).is_none());
}

#[test]
fn copies_id_name_color_status_and_writer() {
    let p = profile_of(&fixture(), "ann");
    assert_eq!(p.id, "ann");
    assert_eq!(p.name, "Anna Řehořová");
    assert_eq!(p.color, "#c0392b");
    assert_eq!(p.status, "review");
    assert_eq!(p.writer, "Petra");
}

#[test]
fn leaves_color_status_writer_and_played_by_empty_when_unset() {
    let p = profile_of(&fixture(), "bob");
    assert_eq!(p.color, "");
    assert_eq!(p.status, "");
    assert_eq!(p.writer, "");
    assert_eq!(p.played_by, "");
    assert_eq!(p.factions, NONE);
}

#[test]
fn names_each_role_as_a_string() {
    let g = fixture();
    assert_eq!(profile_of(&g, "ann").role, "player");
    assert_eq!(profile_of(&g, "cyril").role, "npc");
    let g = game(&[
        ("game.toml", GAME),
        (
            "people/org.toml",
            "[person]\nname = \"Org\"\nrole = \"organizer\"\n",
        ),
    ]);
    assert_eq!(profile_of(&g, "org").role, "organizer");
}

#[test]
fn shows_played_by_as_the_player_name_not_the_id() {
    let p = profile_of(&fixture(), "ann");
    assert_eq!(p.played_by, "Petra Ůlehlová");
}

#[test]
fn shows_factions_by_name() {
    let p = profile_of(&fixture(), "ann");
    assert_eq!(p.factions, ["Řád"]);
}

#[test]
fn copies_goals_and_secrets_in_order() {
    let p = profile_of(&fixture(), "ann");
    assert_eq!(p.goals, ["Přežít", "Najít klíč"]);
    assert_eq!(p.secrets, ["Lže"]);
}

#[test]
fn lists_sheet_sections_as_key_text_pairs() {
    let p = profile_of(&fixture(), "ann");
    // `[person.sheet]` is a sorted map.
    assert_eq!(
        p.sheet,
        pairs(&[("appearance", "Černý plášť"), ("whereabouts", "V kapli")])
    );
}

#[test]
fn lists_relations_by_the_other_persons_name() {
    let p = profile_of(&fixture(), "ann");
    assert_eq!(p.relations, pairs(&[("Bob Čech", "Bratr, kterému nevěří")]));
}

// ---------- seen_by ----------

#[test]
fn seen_by_lists_what_others_think_of_the_person() {
    let p = profile_of(&fixture(), "ann");
    assert_eq!(
        p.seen_by,
        pairs(&[("Bob Čech", "Bojí se jí"), ("Cyril", "Dluží jí")])
    );
}

#[test]
fn seen_by_skips_relations_about_someone_else() {
    let p = profile_of(&fixture(), "cyril");
    assert_eq!(p.seen_by, pairs(&[("Bob Čech", "Neví o něm")]));
}

#[test]
fn seen_by_excludes_the_persons_own_relation_about_themselves() {
    let g = game(&[
        ("game.toml", GAME),
        (
            "people/ann.toml",
            "[person]\nname = \"Ann\"\nrole = \"npc\"\n\n[[person.relation]]\nwith = \"ann\"\ntext = \"Sama sebe\"\n",
        ),
    ]);
    assert_eq!(profile_of(&g, "ann").seen_by, pairs(&[]));
}

// ---------- starts_with ----------

#[test]
fn starts_with_items_and_info_owned_directly_or_through_a_faction() {
    let p = profile_of(&fixture(), "ann");
    // Not Bob's lamp, not the candle nobody starts with.
    assert_eq!(p.starts_with, ["Klíč", "Meč", "Heslo"]);
}

#[test]
fn starts_with_nothing_when_owning_nothing_at_the_start() {
    assert_eq!(profile_of(&fixture(), "cyril").starts_with, NONE);
}

// ---------- timeline: which nodes ----------

#[test]
fn timeline_has_own_nodes_and_faction_nodes_but_not_others() {
    let p = profile_of(&fixture(), "ann");
    let nodes: Vec<&str> = p
        .timeline
        .iter()
        .filter_map(|m| m.node.as_deref())
        .collect();
    assert_eq!(nodes, ["a-hledani", "c-volba", "b-rad", "e-volno"]);
}

#[test]
fn node_moment_has_kind_title_brief_and_text() {
    let p = profile_of(&fixture(), "ann");
    let m = moment(&p, "c-volba");
    assert_eq!(m.kind, "decision");
    assert_eq!(m.title, "Volba");
    assert_eq!(m.brief, "Co uděláš?");
    assert_eq!(m.text, "Org: čekej.");
    assert_eq!(moment(&p, "a-hledani").kind, "action");
}

#[test]
fn with_names_other_people_rooms_and_factions_but_not_the_person() {
    let p = profile_of(&fixture(), "ann");
    assert_eq!(moment(&p, "a-hledani").with, ["Bob Čech", "Kaple", "Řád"]);
    // Through a faction: the faction is named, the person is not in the owners at all.
    assert_eq!(moment(&p, "b-rad").with, ["Řád"]);
    assert_eq!(moment(&p, "e-volno").with, NONE);
}

#[test]
fn decision_lists_its_options_by_label() {
    let p = profile_of(&fixture(), "ann");
    assert_eq!(moment(&p, "c-volba").options, ["Zůstat", "Odejít", "Utéct"]);
    assert_eq!(moment(&p, "a-hledani").options, NONE);
}

#[test]
fn finds_lists_info_the_node_gives() {
    let p = profile_of(&fixture(), "ann");
    assert_eq!(moment(&p, "a-hledani").finds, ["Tip"]);
    assert_eq!(moment(&p, "b-rad").finds, ["Zpráva"]);
}

#[test]
fn told_lists_info_the_node_needs() {
    let p = profile_of(&fixture(), "ann");
    assert_eq!(moment(&p, "a-hledani").told, ["Heslo"]);
    assert_eq!(moment(&p, "b-rad").told, ["Tip"]);
}

#[test]
fn finds_and_told_add_the_option_label_in_parentheses() {
    let p = profile_of(&fixture(), "ann");
    let m = moment(&p, "c-volba");
    assert_eq!(m.finds, ["Konec (Zůstat)"]);
    assert_eq!(m.told, ["Zpráva (Zůstat)"]);
}

#[test]
fn gets_and_uses_list_items_but_not_info() {
    let p = profile_of(&fixture(), "ann");
    let m = moment(&p, "b-rad");
    assert_eq!(m.gets, ["Svíce"]);
    assert_eq!(m.uses, ["Meč"]);
    // `a-hledani` gives only info and uses nothing.
    assert_eq!(moment(&p, "a-hledani").gets, NONE);
    assert_eq!(moment(&p, "a-hledani").uses, NONE);
}

#[test]
fn uses_lists_an_item_once_with_the_labels_of_the_options_that_use_it() {
    let p = profile_of(&fixture(), "ann");
    // "Zůstat" and "Utéct" both use the candle, with "Odejít" (the key) in between: once each,
    // in first-seen order, with the labels of the options that use it.
    assert_eq!(
        moment(&p, "c-volba").uses,
        ["Svíce (Zůstat / Utéct)", "Klíč (Odejít)"]
    );
}

#[test]
fn gets_lists_an_item_once_without_labels_when_every_option_gives_it() {
    let g = game(&[
        ("game.toml", GAME),
        ("items.toml", "[[item]]\nid = \"klic\"\nname = \"Klíč\"\n"),
        (
            "people/ann.toml",
            "[person]\nname = \"Ann\"\nrole = \"player\"\n",
        ),
        (
            "actions/volba.toml",
            "[decision]\ntitle = \"Volba\"\nowners = [\"ann\"]\n\n\
             [[decision.option]]\nlabel = \"A\"\ngives = [\"klic\"]\n\n\
             [[decision.option]]\nlabel = \"B\"\ngives = [\"klic\"]\n",
        ),
    ]);
    assert_eq!(moment(&profile_of(&g, "ann"), "volba").gets, ["Klíč"]);
}

#[test]
fn labels_list_every_option_that_brings_a_thing_when_not_all_do() {
    let g = fixture_with(&[(
        "actions/g-tri.toml",
        "[decision]\ntitle = \"Tři cesty\"\nowners = [\"ann\"]\n\n\
         [[decision.option]]\nlabel = \"Vlevo\"\ngives = [\"tip\"]\n\n\
         [[decision.option]]\nlabel = \"Rovně\"\n\n\
         [[decision.option]]\nlabel = \"Vpravo\"\ngives = [\"tip\"]\n",
    )]);
    let p = profile_of(&g, "ann");
    assert_eq!(moment(&p, "g-tri").finds, ["Tip (Vlevo / Vpravo)"]);
}

#[test]
fn finds_has_no_label_when_every_option_gives_the_info() {
    let g = fixture_with(&[(
        "actions/g-obe.toml",
        "[decision]\ntitle = \"Obě\"\nowners = [\"ann\"]\n\n\
         [[decision.option]]\nlabel = \"A\"\ngives = [\"tip\"]\n\n\
         [[decision.option]]\nlabel = \"B\"\ngives = [\"tip\"]\n",
    )]);
    let p = profile_of(&g, "ann");
    assert_eq!(moment(&p, "g-obe").finds, ["Tip"]);
}

#[test]
fn lists_a_thing_once_when_an_action_names_it_twice() {
    let g = fixture_with(&[(
        "actions/g-dvakrat.toml",
        "[action]\ntitle = \"Dvakrát\"\nowners = [\"ann\"]\ngives = [\"svice\", \"tip\", \"svice\", \"tip\"]\n",
    )]);
    let p = profile_of(&g, "ann");
    let m = moment(&p, "g-dvakrat");
    assert_eq!(m.gets, ["Svíce"]);
    assert_eq!(m.finds, ["Tip"]);
}

// ---------- timeline: when ----------

#[test]
fn when_is_the_segment_name_for_a_segment_node() {
    let p = profile_of(&fixture(), "ann");
    assert_eq!(moment(&p, "b-rad").when, "Ráno ů");
}

#[test]
fn when_is_cycle_name_and_step_for_a_cycle_node() {
    let p = profile_of(&fixture(), "ann");
    assert_eq!(moment(&p, "a-hledani").when, "Annin cyklus · 1. Ty");
    assert_eq!(moment(&p, "c-volba").when, "Annin cyklus · 2. Potřeba");
}

#[test]
fn when_is_empty_and_at_is_none_for_a_node_without_time() {
    let p = profile_of(&fixture(), "ann");
    let m = moment(&p, "e-volno");
    assert_eq!(m.when, "");
    assert_eq!(m.at, None);
}

#[test]
fn at_is_the_first_tick_of_the_node() {
    let p = profile_of(&fixture(), "ann");
    assert_eq!(moment(&p, "a-hledani").at, Some(0));
    assert_eq!(moment(&p, "c-volba").at, Some(1));
    assert_eq!(moment(&p, "b-rad").at, Some(2));
}

// ---------- timeline: life moments ----------

#[test]
fn enters_phase_and_dies_by_segment_use_the_segment_name() {
    let p = profile_of(&fixture(), "cyril");
    let kinds: Vec<(&str, &str)> = p
        .timeline
        .iter()
        .map(|m| (m.kind, m.when.as_str()))
        .collect();
    assert_eq!(
        kinds,
        [
            ("enters", "Ráno ů"),
            ("phase", "Ráno ů"),
            ("action", "Ráno ů"),
            ("dies", "Ráno ů"),
        ]
    );
}

#[test]
fn life_moments_have_no_node_and_no_title() {
    let p = profile_of(&fixture(), "cyril");
    for kind in ["enters", "phase", "dies"] {
        let m = life(&p, kind);
        assert_eq!(m.node, None, "{kind}");
        assert_eq!(m.title, "", "{kind}");
    }
}

#[test]
fn enters_at_the_start_and_dies_at_the_last_tick_of_the_segment() {
    let p = profile_of(&fixture(), "cyril");
    assert_eq!(life(&p, "enters").at, Some(2));
    assert_eq!(life(&p, "phase").at, Some(2));
    assert_eq!(life(&p, "dies").at, Some(3));
}

#[test]
fn step_phase_and_step_death_use_the_hero_cycle_name() {
    let p = profile_of(&fixture(), "ann");
    assert_eq!(life(&p, "phase").when, "Annin cyklus · 2. Potřeba");
    assert_eq!(life(&p, "dies").when, "Annin cyklus · 2. Potřeba");
}

#[test]
fn phase_brief_and_text_are_its_goals_and_secrets_joined() {
    let p = profile_of(&fixture(), "ann");
    let m = life(&p, "phase");
    assert_eq!(m.brief, "Utéct · Skrýt klíč");
    assert_eq!(m.text, "Zradila · Má dluh");
}

#[test]
fn phase_without_secrets_has_empty_text() {
    let p = profile_of(&fixture(), "cyril");
    let m = life(&p, "phase");
    assert_eq!(m.brief, "Hlídat");
    assert_eq!(m.text, "");
}

#[test]
fn no_enters_moment_without_enters() {
    let p = profile_of(&fixture(), "ann");
    assert!(p.timeline.iter().all(|m| m.kind != "enters"));
}

#[test]
fn no_life_moments_for_a_person_without_enters_phases_or_death() {
    let p = profile_of(&fixture(), "bob");
    assert!(
        p.timeline
            .iter()
            .all(|m| !matches!(m.kind, "enters" | "phase" | "dies"))
    );
}

#[test]
fn enters_after_a_predecessor_when_they_die() {
    let g = fixture_with(&[(
        "people/dana.toml",
        "[person]\nname = \"Dana\"\nrole = \"npc\"\nafter = \"ann\"\n",
    )]);
    let p = profile_of(&g, "dana");
    let m = life(&p, "enters");
    assert_eq!(m.when, "← Anna Řehořová");
    // Ann dies at the end of step 2 of her cycle, tick 1; Dana starts at tick 2.
    assert_eq!(m.at, Some(2));
}

#[test]
fn keeps_only_a_valid_rrggbb_color() {
    let mut g = fixture();
    let ann = g.people.get_mut(&id("ann")).unwrap();
    *ann.color.as_mut().unwrap().get_mut() = "red; } body { x".to_owned();
    assert_eq!(profile_of(&g, "ann").color, "");
    for bad in ["#c0392", "#c0392bb", "c0392b0", "#g0392b"] {
        let ann = g.people.get_mut(&id("ann")).unwrap();
        *ann.color.as_mut().unwrap().get_mut() = bad.to_owned();
        assert_eq!(profile_of(&g, "ann").color, "", "{bad}");
    }
    let ann = g.people.get_mut(&id("ann")).unwrap();
    *ann.color.as_mut().unwrap().get_mut() = "#A1b2C3".to_owned();
    assert_eq!(profile_of(&g, "ann").color, "#A1b2C3");
}

// ---------- timeline: order ----------

#[test]
fn sorts_a_shorter_node_first_at_the_same_start() {
    // `a-dlouha` takes the whole first segment (ticks 0..2), `a-hledani` only step 1 (0..1);
    // by id `a-dlouha` comes first.
    let g = fixture_with(&[(
        "actions/a-dlouha.toml",
        "[action]\ntitle = \"Dlouhá\"\nsegment = \"s1\"\nowners = [\"ann\"]\n",
    )]);
    let p = profile_of(&g, "ann");
    let nodes: Vec<&str> = p
        .timeline
        .iter()
        .filter_map(|m| m.node.as_deref())
        .collect();
    assert_eq!(nodes[..2], ["a-hledani", "a-dlouha"]);
}

#[test]
fn sorts_by_tick_with_untimed_last_and_life_order_at_the_same_tick() {
    let p = profile_of(&fixture(), "ann");
    let order: Vec<(&str, Option<u32>)> = p.timeline.iter().map(|m| (m.kind, m.at)).collect();
    assert_eq!(
        order,
        [
            ("action", Some(0)),
            // Tick 1: phase < node < dies.
            ("phase", Some(1)),
            ("decision", Some(1)),
            ("dies", Some(1)),
            ("action", Some(2)),
            ("action", None),
        ]
    );
}

#[test]
fn sorts_enters_before_phase_before_node_before_dies_at_the_same_tick() {
    // Cyril enters, changes phase, acts and dies in the same segment; the order of the
    // `[[person.phase]]` and file order must not matter.
    let p = profile_of(&fixture(), "cyril");
    let kinds: Vec<&str> = p.timeline.iter().map(|m| m.kind).collect();
    assert_eq!(kinds, ["enters", "phase", "action", "dies"]);
}

// ---------- games without segments ----------

#[test]
fn has_only_node_moments_without_time_when_the_game_has_no_segments() {
    let g = game(&[
        ("game.toml", GAME),
        (
            "people/ann.toml",
            "[person]\nname = \"Ann\"\nrole = \"player\"\n",
        ),
        (
            "actions/a.toml",
            "[action]\ntitle = \"A\"\nowners = [\"ann\"]\n",
        ),
    ]);
    let p = profile_of(&g, "ann");
    let order: Vec<(&str, &str, Option<u32>)> = p
        .timeline
        .iter()
        .map(|m| (m.kind, m.when.as_str(), m.at))
        .collect();
    assert_eq!(order, [("action", "", None)]);
}

/// Pins current behavior: without segments `check` skips the lives rules, so a step death and a
/// phase with neither `segment` nor `step` load without a diagnostic (`enters` cannot be set: it
/// names a segment). They show as untimed moments, ordered phase < node < dies.
#[test]
fn keeps_phases_and_step_deaths_untimed_when_the_game_has_no_segments() {
    let g = game(&[
        ("game.toml", GAME),
        (
            "people/ann.toml",
            "[person]\nname = \"Ann\"\nrole = \"npc\"\ndies = 1\n\n[[person.phase]]\ngoals = [\"Ř\", \"Ů\"]\n",
        ),
        (
            "actions/a.toml",
            "[action]\ntitle = \"A\"\nowners = [\"ann\"]\n",
        ),
    ]);
    let p = profile_of(&g, "ann");
    let order: Vec<(&str, &str, &str, Option<u32>)> = p
        .timeline
        .iter()
        .map(|m| (m.kind, m.when.as_str(), m.brief.as_str(), m.at))
        .collect();
    assert_eq!(
        order,
        [
            ("phase", "", "Ř · Ů", None),
            ("action", "", "", None),
            // No hero cycle: just the step and its name.
            ("dies", "1. Ty", "", None),
        ]
    );
}

#[test]
fn life_moments_end_one_tick_after_they_start() {
    let p = profile_of(&fixture(), "cyril");
    for kind in ["enters", "phase", "dies"] {
        let m = life(&p, kind);
        assert_eq!(m.end, m.at.map(|a| a + 1), "{kind}");
    }
    assert_eq!(life(&p, "enters").end, Some(3));
}

#[test]
fn node_moment_ends_where_its_span_ends() {
    let p = profile_of(&fixture(), "ann");
    assert_eq!(moment(&p, "a-hledani").end, Some(1));
    assert_eq!(moment(&p, "b-rad").end, Some(4));
    assert_eq!(moment(&p, "e-volno").end, None);
}

#[test]
fn sorts_death_after_a_longer_node_that_starts_at_the_same_tick() {
    // Three steps, so a segment is 3 ticks. Xena's cycle spans both segments (2 ticks a step):
    // step 2 is ticks 2..4. Zora dies with the first segment, at tick 2 (life 0..3); she is in
    // step 2, which starts at her death's tick but ends later. The death still comes last.
    let g = game(&[
        (
            "game.toml",
            "[game]\ntitle = \"T\"\nlanguage = \"cs\"\nsteps = [\"A\", \"B\", \"C\"]\n",
        ),
        ("segments.toml", SEGMENTS),
        (
            "cycles.toml",
            "[[cycle]]\nid = \"c-x\"\nname = \"X\"\nhero = \"xena\"\nfrom = \"s1\"\nto = \"s2\"\n",
        ),
        (
            "people/xena.toml",
            "[person]\nname = \"Xena\"\nrole = \"npc\"\ndies = 3\n",
        ),
        (
            "people/zora.toml",
            "[person]\nname = \"Zora\"\nrole = \"npc\"\ndies = \"s1\"\n",
        ),
        (
            "actions/krok.toml",
            "[action]\ntitle = \"Krok\"\ncycle = \"c-x\"\nstep = 2\nowners = [\"xena\", \"zora\"]\n",
        ),
    ]);
    let p = profile_of(&g, "zora");
    let order: Vec<(&str, Option<u32>, Option<u32>)> =
        p.timeline.iter().map(|m| (m.kind, m.at, m.end)).collect();
    assert_eq!(
        order,
        [("action", Some(2), Some(4)), ("dies", Some(2), Some(3))]
    );
}
