//! The Balance tab (`balance`): numbers per player character, from small in-memory games.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used, clippy::panic)]

use std::fmt::Write as _;

use larp_graph::{Balance, balance};
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

fn of<'a>(all: &'a [Balance], id: &str) -> &'a Balance {
    all.iter()
        .find(|b| b.id == id)
        .unwrap_or_else(|| panic!("{id} is not in the balance: {all:?}"))
}

const GAME: &str = "[game]\ntitle = \"Opatství\"\nlanguage = \"cs\"\n";

/// Segments in file order: `one` (Příjezd), `two` (Noc), `three` (Ráno); names are not in
/// alphabetical order, so the order of `idle` shows which order is used.
const SEGMENTS: &str = "[[segment]]\nid = \"one\"\nname = \"Příjezd\"\n\n\
    [[segment]]\nid = \"two\"\nname = \"Noc\"\n\n\
    [[segment]]\nid = \"three\"\nname = \"Ráno\"\n";

/// The faction `bratri` (Anna and the NPC Bob) with 2 goals and 1 secret. Its goals and sheet
/// have words, which must not count as Anna's own words.
const FACTIONS: &str = "[[faction]]\nid = \"bratri\"\nname = \"Konvent\"\n\
    goals = [\"Chránit kryptu před všemi\", \"Mlčet\"]\nsecrets = [\"Opat žije\"]\n\n\
    [faction.sheet]\nhistory = \"Konvent stojí na kopci tři sta let.\"\n";

const ROOMS: &str = "[[room]]\nid = \"crypt\"\nname = \"Krypta\"\n";

/// `key` starts with the faction, `seal` with Anna, `bones` with the crypt, `ring` is Anna's but
/// not starting; `lamp` is given by two actions, `candle` only by the second decision option.
const ITEMS: &str = "[[item]]\nid = \"key\"\nname = \"Klíč\"\nowners = [\"bratri\"]\nstarting = true\n\n\
    [[item]]\nid = \"seal\"\nname = \"Pečeť\"\nowners = [\"anna\"]\nstarting = true\n\n\
    [[item]]\nid = \"bones\"\nname = \"Kosti\"\nowners = [\"crypt\"]\nstarting = true\n\n\
    [[item]]\nid = \"ring\"\nname = \"Prsten\"\nowners = [\"anna\"]\n\n\
    [[item]]\nid = \"lamp\"\nname = \"Lampa\"\n\n\
    [[item]]\nid = \"candle\"\nname = \"Svíčka\"\n";

/// `rumor` starts with Anna, `card` with the faction; `letter` and `end` are given by nodes.
const INFO: &str = "[[info]]\nid = \"rumor\"\nname = \"Fáma\"\nowners = [\"anna\"]\nstarting = true\n\n\
    [[info]]\nid = \"card\"\nname = \"Karta\"\nowners = [\"bratri\"]\nstarting = true\n\n\
    [[info]]\nid = \"letter\"\nname = \"Dopis\"\n\n\
    [[info]]\nid = \"end\"\nname = \"Konec\"\nending = true\n";

/// Anna: player in `bratri`; 1 goal, 1 secret, 2 sheet sections, a relation to Marta.
/// Her own words: 2 + 2 + 3 + 2 (with a no-break space) + 3 = 12.
const ANNA: &str = "[person]\nname = \"Anna Řeháková\"\nrole = \"player\"\nfactions = [\"bratri\"]\n\
    goals = [\"Najít klíč\"]\nsecrets = [\"Zradila opata\"]\n\n\
    [person.sheet]\nbackstory = \"Žila  v\tklášteře.\"\nwhereabouts = \"V\u{a0}kapli.\"\n\n\
    [[person.relation]]\nwith = \"marta\"\ntext = \"Bojí se jí.\"\n";

/// Marta: player, relations to Anna, Petra and herself.
const MARTA: &str = "[person]\nname = \"Marta Čechová\"\nrole = \"player\"\ngoals = [\"Utéct\"]\n\n\
    [[person.relation]]\nwith = \"anna\"\ntext = \"Věří jí.\"\n\n\
    [[person.relation]]\nwith = \"petra\"\ntext = \"Šéfová.\"\n\n\
    [[person.relation]]\nwith = \"marta\"\ntext = \"Nenávidí se.\"\n";

/// Zdeněk: a player with nothing at all.
const ZDENEK: &str = "[person]\nname = \"Zdeněk\"\nrole = \"player\"\n";

const PETRA: &str = "[person]\nname = \"Petra\"\nrole = \"organizer\"\n\n\
    [[person.relation]]\nwith = \"anna\"\ntext = \"Hlídat.\"\n";

const BOB: &str = "[person]\nname = \"Bratr Bob\"\nrole = \"npc\"\nfactions = [\"bratri\"]\n\n\
    [[person.relation]]\nwith = \"anna\"\ntext = \"Sestra.\"\n";

/// - `a-arrive` (one): Anna and Petra; gives `lamp` and `letter`.
/// - `b-crypt` (two): the faction and the crypt; needs `lamp`, `rumor`, `card`.
/// - `c-choice` (two): decision of Anna and Marta; option 1 needs `letter`, option 2 gives
///   `candle` and `end`.
/// - `d-marta` (no segment): Marta; gives `lamp`.
const ACTIONS: [(&str, &str); 4] = [
    (
        "actions/a-arrive.toml",
        "[action]\ntitle = \"Příjezd\"\nsegment = \"one\"\nowners = [\"anna\", \"petra\"]\ngives = [\"lamp\", \"letter\"]\n",
    ),
    (
        "actions/b-crypt.toml",
        "[action]\ntitle = \"Krypta\"\nsegment = \"two\"\nowners = [\"bratri\", \"crypt\"]\nneeds = [\"lamp\", \"rumor\", \"card\"]\n",
    ),
    (
        "actions/c-choice.toml",
        "[decision]\ntitle = \"Volba\"\nsegment = \"two\"\nowners = [\"anna\", \"marta\"]\n\n\
         [[decision.option]]\nlabel = \"Číst\"\nneeds = [\"letter\"]\n\n\
         [[decision.option]]\nlabel = \"Zapálit\"\ngives = [\"candle\", \"end\"]\n",
    ),
    (
        "actions/d-marta.toml",
        "[action]\ntitle = \"Martina cesta\"\nowners = [\"marta\"]\ngives = [\"lamp\"]\n",
    ),
];

/// People files are listed out of id order on purpose.
fn abbey() -> Game {
    let mut files = vec![
        ("game.toml", GAME),
        ("segments.toml", SEGMENTS),
        ("factions.toml", FACTIONS),
        ("rooms.toml", ROOMS),
        ("items.toml", ITEMS),
        ("info.toml", INFO),
        ("people/zdenek.toml", ZDENEK),
        ("people/petra.toml", PETRA),
        ("people/marta.toml", MARTA),
        ("people/bob.toml", BOB),
        ("people/anna.toml", ANNA),
    ];
    files.extend(ACTIONS);
    game(&files)
}

// ---------- which people ----------

#[test]
fn lists_only_players_in_id_order() {
    let b = balance(&abbey());
    let ids: Vec<&str> = b.iter().map(|b| b.id.as_str()).collect();
    assert_eq!(ids, ["anna", "marta", "zdenek"]);
}

#[test]
fn carries_the_name_with_diacritics() {
    let b = balance(&abbey());
    assert_eq!(of(&b, "anna").name, "Anna Řeháková");
    assert_eq!(of(&b, "marta").name, "Marta Čechová");
}

#[test]
fn is_empty_when_the_game_has_no_players() {
    let g = game(&[
        ("game.toml", GAME),
        (
            "people/petra.toml",
            "[person]\nname = \"Petra\"\nrole = \"organizer\"\n",
        ),
    ]);
    assert_eq!(balance(&g), []);
}

// ---------- actions and decisions ----------

#[test]
fn counts_actions_owned_directly_or_through_a_faction() {
    let b = balance(&abbey());
    assert_eq!(of(&b, "anna").actions, 2, "a-arrive and b-crypt (faction)");
    assert_eq!(of(&b, "marta").actions, 1, "d-marta");
    assert_eq!(of(&b, "zdenek").actions, 0);
}

#[test]
fn counts_decisions_apart_from_actions() {
    let b = balance(&abbey());
    assert_eq!(of(&b, "anna").decisions, 1);
    assert_eq!(of(&b, "marta").decisions, 1);
    assert_eq!(of(&b, "zdenek").decisions, 0);
}

// ---------- goals and secrets ----------

#[test]
fn counts_goals_with_the_factions_goals() {
    let b = balance(&abbey());
    assert_eq!(of(&b, "anna").goals, 3, "1 own + 2 of the faction");
    assert_eq!(of(&b, "marta").goals, 1);
}

#[test]
fn counts_a_faction_listed_twice_once() {
    let g = game(&[
        ("game.toml", GAME),
        ("factions.toml", FACTIONS),
        (
            "people/zdenek.toml",
            "[person]\nname = \"Zdeněk\"\nrole = \"player\"\nfactions = [\"bratri\", \"bratri\"]\n",
        ),
    ]);
    let b = balance(&g);
    assert_eq!(of(&b, "zdenek").goals, 2);
    assert_eq!(of(&b, "zdenek").secrets, 1);
}

#[test]
fn counts_secrets_with_the_factions_secrets() {
    let b = balance(&abbey());
    assert_eq!(of(&b, "anna").secrets, 2, "1 own + 1 of the faction");
    assert_eq!(of(&b, "marta").secrets, 0);
}

// ---------- relations ----------

#[test]
fn counts_own_relations_out_without_one_to_oneself() {
    let b = balance(&abbey());
    assert_eq!(of(&b, "anna").relations_out, 1);
    assert_eq!(
        of(&b, "marta").relations_out,
        2,
        "Anna and Petra, not herself"
    );
}

/// Anna has two relations to Marta and one to Petra; Petra has two relations to Anna.
fn doubled() -> Game {
    game(&[
        ("game.toml", GAME),
        (
            "people/anna.toml",
            "[person]\nname = \"Anna\"\nrole = \"player\"\n\n\
             [[person.relation]]\nwith = \"marta\"\ntext = \"Věří jí.\"\n\n\
             [[person.relation]]\nwith = \"marta\"\ntext = \"Ale ne úplně.\"\n\n\
             [[person.relation]]\nwith = \"petra\"\ntext = \"Šéfová.\"\n",
        ),
        (
            "people/marta.toml",
            "[person]\nname = \"Marta\"\nrole = \"player\"\n\n\
             [[person.relation]]\nwith = \"anna\"\ntext = \"Sestra.\"\n",
        ),
        (
            "people/petra.toml",
            "[person]\nname = \"Petra\"\nrole = \"organizer\"\n\n\
             [[person.relation]]\nwith = \"anna\"\ntext = \"Hlídat.\"\n\n\
             [[person.relation]]\nwith = \"anna\"\ntext = \"Pozor.\"\n",
        ),
    ])
}

#[test]
fn counts_relations_out_once_per_other_person() {
    assert_eq!(of(&balance(&doubled()), "anna").relations_out, 2);
}

#[test]
fn counts_relations_in_once_per_other_person() {
    let b = balance(&doubled());
    assert_eq!(of(&b, "anna").relations_in, 2, "Marta and Petra");
    assert_eq!(of(&b, "marta").relations_in, 1, "Anna, with two relations");
}

#[test]
fn counts_relations_in_from_players_npcs_and_organizers() {
    let b = balance(&abbey());
    assert_eq!(of(&b, "anna").relations_in, 3, "Marta, Petra and Bob");
}

#[test]
fn does_not_count_a_relation_to_oneself_as_relation_in() {
    let b = balance(&abbey());
    assert_eq!(of(&b, "marta").relations_in, 1, "only Anna's");
}

// ---------- meets ----------

#[test]
fn meets_people_and_faction_members_but_not_rooms_or_oneself() {
    let b = balance(&abbey());
    // Petra (a-arrive), Bob (via the faction in b-crypt), Marta (c-choice); the crypt and Anna
    // herself (owner of a-arrive, member of the faction) do not count.
    assert_eq!(of(&b, "anna").meets, 3);
}

#[test]
fn meets_each_person_once() {
    // Anna and Marta share two nodes; Marta still meets Anna once.
    let mut files = vec![
        ("game.toml", GAME),
        (
            "people/anna.toml",
            "[person]\nname = \"Anna\"\nrole = \"player\"\n",
        ),
        (
            "people/marta.toml",
            "[person]\nname = \"Marta\"\nrole = \"player\"\n",
        ),
    ];
    files.push((
        "actions/a.toml",
        "[action]\ntitle = \"A\"\nowners = [\"anna\", \"marta\"]\n",
    ));
    files.push((
        "actions/b.toml",
        "[action]\ntitle = \"B\"\nowners = [\"marta\", \"anna\"]\n",
    ));
    let b = balance(&game(&files));
    assert_eq!(of(&b, "marta").meets, 1);
}

#[test]
fn meets_nobody_alone() {
    let b = balance(&abbey());
    assert_eq!(of(&b, "zdenek").meets, 0);
}

// ---------- knowledge ----------

#[test]
fn knows_at_start_own_and_faction_info() {
    let b = balance(&abbey());
    assert_eq!(of(&b, "anna").knows_at_start, 2, "rumor and card");
    assert_eq!(of(&b, "marta").knows_at_start, 0);
}

#[test]
fn learns_distinct_info_found_or_told_but_not_known_at_start() {
    let b = balance(&abbey());
    // letter (found at a-arrive and told at c-choice: once) and end (found); rumor and card are
    // told at b-crypt but she starts with them.
    assert_eq!(of(&b, "anna").learns, 2);
}

#[test]
fn learns_info_that_is_only_told() {
    let b = balance(&abbey());
    assert_eq!(of(&b, "marta").learns, 2, "letter (told) and end (found)");
    assert_eq!(of(&b, "zdenek").learns, 0);
}

// ---------- items ----------

#[test]
fn counts_distinct_items_held_or_given_but_no_info() {
    let b = balance(&abbey());
    // key (faction, starting), seal (starting), lamp (a-arrive and b-crypt needs it, given once),
    // candle (second option); not ring (not starting), bones (the crypt's), letter/end (info).
    assert_eq!(of(&b, "anna").items, 4);
}

#[test]
fn counts_items_given_by_any_option_and_by_untimed_actions() {
    let b = balance(&abbey());
    assert_eq!(of(&b, "marta").items, 2, "lamp (d-marta) and candle");
    assert_eq!(of(&b, "zdenek").items, 0);
}

#[test]
fn counts_an_item_given_twice_once() {
    let files = [
        ("game.toml", GAME),
        (
            "people/anna.toml",
            "[person]\nname = \"Anna\"\nrole = \"player\"\n",
        ),
        (
            "items.toml",
            "[[item]]\nid = \"lamp\"\nname = \"Lampa\"\nowners = [\"anna\"]\nstarting = true\n",
        ),
        (
            "actions/a.toml",
            "[action]\ntitle = \"A\"\nowners = [\"anna\"]\ngives = [\"lamp\"]\n",
        ),
        (
            "actions/b.toml",
            "[action]\ntitle = \"B\"\nowners = [\"anna\"]\ngives = [\"lamp\"]\n",
        ),
    ];
    let b = balance(&game(&files));
    assert_eq!(of(&b, "anna").items, 1);
}

// ---------- words ----------

#[test]
fn counts_words_of_own_goals_secrets_sheet_and_relations_in_czech() {
    let b = balance(&abbey());
    // "Najít klíč" 2, "Zradila opata" 2, "Žila  v\tklášteře." 3, "V\u{a0}kapli." 2,
    // "Bojí se jí." 3; the faction's texts are not hers.
    assert_eq!(of(&b, "anna").words, 12);
}

#[test]
fn counts_words_of_phase_goals_secrets_and_sheet() {
    let g = game(&[
        ("game.toml", GAME),
        ("segments.toml", SEGMENTS),
        (
            "people/zdenek.toml",
            "[person]\nname = \"Zdeněk\"\nrole = \"player\"\ngoals = [\"Přežít\"]\n\n\
             [[person.phase]]\nsegment = \"two\"\ngoals = [\"Utéct z kláštera\"]\n\
             secrets = [\"Ukradl prsten\"]\n\n\
             [person.phase.sheet]\nwhereabouts = \"Schováváš se ve sklepě.\"\n\n\
             [[person.phase]]\nsegment = \"three\"\nsecrets = [\"Lže\"]\n",
        ),
    ]);
    // 1 own + 3 + 2 + 4 in the first phase + 1 in the second.
    assert_eq!(of(&balance(&g), "zdenek").words, 11);
}

#[test]
fn counts_words_of_every_relation() {
    let b = balance(&abbey());
    assert_eq!(of(&b, "marta").words, 1 + 2 + 1 + 2);
    assert_eq!(of(&b, "zdenek").words, 0);
}

// ---------- idle ----------

#[test]
fn idle_lists_segments_without_own_nodes_in_segment_order() {
    let b = balance(&abbey());
    assert_eq!(of(&b, "anna").idle, ["Ráno"]);
    assert_eq!(of(&b, "zdenek").idle, ["Příjezd", "Noc", "Ráno"]);
}

#[test]
fn idle_is_empty_when_an_own_node_has_no_time() {
    // d-marta has no segment, so Marta could be busy anywhere.
    assert_eq!(of(&balance(&abbey()), "marta").idle, Vec::<String>::new());
}

#[test]
fn idle_is_empty_when_the_game_has_no_segments() {
    let g = game(&[("game.toml", GAME), ("people/zdenek.toml", ZDENEK)]);
    let b = balance(&g);
    assert_eq!(of(&b, "zdenek").idle, Vec::<String>::new());
}

#[test]
fn idle_skips_segments_before_entering_and_after_dying() {
    let g = game(&[
        ("game.toml", GAME),
        ("segments.toml", SEGMENTS),
        (
            "people/zdenek.toml",
            "[person]\nname = \"Zdeněk\"\nrole = \"player\"\nenters = \"two\"\ndies = \"two\"\n",
        ),
    ]);
    assert_eq!(of(&balance(&g), "zdenek").idle, ["Noc"]);
}

#[test]
fn idle_runs_to_the_end_after_entering() {
    let g = game(&[
        ("game.toml", GAME),
        ("segments.toml", SEGMENTS),
        (
            "people/zdenek.toml",
            "[person]\nname = \"Zdeněk\"\nrole = \"player\"\nenters = \"two\"\n",
        ),
    ]);
    assert_eq!(of(&balance(&g), "zdenek").idle, ["Noc", "Ráno"]);
}

/// Three steps (3 ticks per segment). Ann's cycle runs over `from`..`to`; her actions are at the
/// given steps.
fn cycled(from: &str, to: &str, steps: &[u8], dies: Option<u8>) -> Game {
    let mut ann =
        "[person]\nname = \"Anna Řeháková\"\nrole = \"player\"\nplayed_by = \"p1\"\n".to_owned();
    if let Some(d) = dies {
        writeln!(ann, "dies = {d}").unwrap();
    }
    let cycles = format!(
        "[[cycle]]\nid = \"c1\"\nname = \"Annin cyklus\"\nhero = \"ann\"\nfrom = \"{from}\"\nto = \"{to}\"\n"
    );
    let actions: Vec<(String, String)> = steps
        .iter()
        .map(|s| {
            (
                format!("actions/k{s}.toml"),
                format!("[action]\ntitle = \"Krok {s}\"\ncycle = \"c1\"\nstep = {s}\nowners = [\"ann\"]\n"),
            )
        })
        .collect();
    let mut files: Vec<(&str, &str)> = vec![
        (
            "game.toml",
            "[game]\ntitle = \"Koloběh\"\nlanguage = \"cs\"\nsteps = [\"Ty\", \"Potřeba\", \"Odchod\"]\n",
        ),
        ("segments.toml", SEGMENTS),
        ("players.toml", "[[player]]\nid = \"p1\"\nname = \"Pat\"\n"),
        ("cycles.toml", &cycles),
        ("people/ann.toml", &ann),
    ];
    files.extend(actions.iter().map(|(p, t)| (p.as_str(), t.as_str())));
    game(&files)
}

#[test]
fn idle_counts_a_cycle_step_in_the_segment_it_falls_in() {
    // Cycle over "two" only: each step is one tick inside Noc.
    let g = cycled("two", "two", &[1], None);
    assert_eq!(of(&balance(&g), "ann").idle, ["Příjezd", "Ráno"]);
}

#[test]
fn idle_counts_a_step_spanning_two_segments_in_both() {
    // Cycle over "one".."two" (ticks 0..6, 2 per step): step 2 is ticks 2..4, the end of Příjezd
    // and the start of Noc.
    let g = cycled("one", "two", &[2], None);
    assert_eq!(of(&balance(&g), "ann").idle, ["Ráno"]);
}

#[test]
fn idle_ends_at_a_death_step() {
    // Cycle over "two".."three" (ticks 3..9); she dies at step 1 (tick 3..5): alive in Příjezd
    // and Noc only, and busy in Noc.
    let g = cycled("two", "three", &[1], Some(1));
    assert_eq!(of(&balance(&g), "ann").idle, ["Příjezd"]);
}

// ---------- the whole report ----------

#[test]
fn graph_data_carries_the_balance() {
    let g = abbey();
    let data = larp_graph::build(&g, &[], None);
    assert_eq!(data.balance, balance(&g));
}
