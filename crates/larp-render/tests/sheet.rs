//! Rendering player sheets.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used)]

use larp_model::{Game, Id, SourceFile, load};

fn game(person: &str) -> Game {
    let files = [
        ("game.toml", "[game]\ntitle = \"Test\"\nlanguage = \"cs\"\n"),
        ("people/ann.toml", person),
        (
            "segments.toml",
            "[[segment]]\nid = \"s1\"\nname = \"Ty\"\n\n[[segment]]\nid = \"s2\"\nname = \"Potřeba\"\n\n[[segment]]\nid = \"s3\"\nname = \"Odchod\"\n",
        ),
        (
            "players.toml",
            "[[player]]\nid = \"p1\"\nname = \"Petra\"\n",
        ),
        (
            "actions/talk.toml",
            "[decision]\ntitle = \"Talk\"\nowners = [\"ann\"]\nbrief = \"Speak up.\"\n\n[[decision.option]]\nlabel = \"Yes\"\n",
        ),
    ];
    let files: Vec<SourceFile> = files
        .iter()
        .map(|(p, t)| SourceFile {
            path: (*p).to_owned(),
            text: (*t).to_owned(),
        })
        .collect();
    load(&files).game.unwrap()
}

fn render(person: &str) -> String {
    let g = game(person);
    let id = Id::parse("ann").unwrap();
    let graph = larp_graph::build(&g, &[], None);
    let destiny = larp_graph::destiny(&g, &graph, &id);
    larp_render::player_sheet_html(&g, &id, &destiny, None).unwrap()
}

const ANN: &str = "[person]\nname = \"Anna Nováková\"\nrole = \"player\"\ngoals = [\"Přežít\"]\n\n[person.sheet]\nwhereabouts = \"V kuchyni.\"\nbackstory = \"Kuchařka.\"\nsmell = \"Česnek\"\n";

#[test]
fn sheet_has_both_parts_in_czech() {
    let html = render(ANN);
    assert!(html.contains("Anna Nováková"));
    assert!(html.contains("Kdo jsi"));
    assert!(html.contains("Tvůj osud"));
    assert!(html.contains("V kuchyni."));
    assert!(html.contains("Speak up."));
    assert!(html.contains("Tvé rozhodnutí"));
}

#[test]
fn unknown_sheet_keys_become_extra_sections() {
    let html = render(ANN);
    assert!(html.contains(">smell</h3>"));
    assert!(html.contains("Česnek"));
}

#[test]
fn user_text_is_html_escaped() {
    let evil = ANN.replace("Kuchařka.", "<script>alert(1)</script>");
    let html = render(&evil);
    assert!(!html.contains("<script>alert(1)"));
    assert!(html.contains("&lt;script&gt;"));
}

#[test]
fn custom_template_is_used() {
    let g = game(ANN);
    let id = Id::parse("ann").unwrap();
    let graph = larp_graph::build(&g, &[], None);
    let destiny = larp_graph::destiny(&g, &graph, &id);
    let html =
        larp_render::player_sheet_html(&g, &id, &destiny, Some("{{ person.name }}|{{ t.part2 }}"))
            .unwrap();
    assert_eq!(html, "Anna Nováková|Tvůj osud");
}

#[test]
fn broken_template_is_a_clear_error() {
    let g = game(ANN);
    let id = Id::parse("ann").unwrap();
    let graph = larp_graph::build(&g, &[], None);
    let destiny = larp_graph::destiny(&g, &graph, &id);
    let err = larp_render::player_sheet_html(&g, &id, &destiny, Some("{% if %}")).unwrap_err();
    assert!(err.to_string().starts_with("template error"));
}

#[test]
fn life_and_phases_are_shown_in_segment_order() {
    let person = ANN.replace(
        "[person.sheet]",
        "played_by = \"p1\"\nenters = \"s1\"\ndies = \"s3\"\n\n[person.sheet]",
    ) + "\n[[person.phase]]\nsegment = \"s3\"\ngoals = [\"Utéct\"]\n\n\
         [[person.phase]]\nsegment = \"s2\"\n[person.phase.sheet]\nwhereabouts = \"Na nádraží <sama>.\"\n";
    let html = render(&person);
    assert!(html.contains("Hraje:</b> Petra"), "{html}");
    assert!(html.contains("Umíráš:</b> Odchod"), "{html}");
    assert!(html.contains("Jak se měníš"), "{html}");
    let second = html.find("Od: Potřeba").unwrap();
    let third = html.find("Od: Odchod").unwrap();
    assert!(second < third, "phases must follow segment order");
    assert!(html.contains("Na nádraží &lt;sama&gt;."), "{html}");
    assert!(html.contains("Utéct"));
    assert!(
        html.contains("Kde jsi, když vstupuješ do příběhu"),
        "{html}"
    );
}

#[test]
fn sheet_without_life_has_no_life_line_or_part_three() {
    let html = render(ANN);
    assert!(!html.contains("class=\"life\""));
    assert!(!html.contains("Jak se měníš"));
}

#[test]
fn outside_cycle_and_played_by_alone() {
    let person = ANN.replace(
        "[person.sheet]",
        "played_by = \"p1\"\noutside_cycle = true\n\n[person.sheet]",
    );
    let html = render(&person);
    assert!(html.contains("mimo cyklus"), "{html}");
    assert!(html.contains("Hraje:</b> Petra"), "{html}");
    assert!(!html.contains("Umíráš"), "{html}");
    assert!(html.contains("Kde jsi na začátku hry"), "{html}");
}

#[test]
fn user_sheet_keys_are_not_swapped_for_ui_labels() {
    let html = render(&ANN.replace("smell = ", "part1 = \"x\"\nsmell = "));
    assert!(html.contains("<h3>part1</h3>"), "{html}");
}

/// Renders Ann's sheet in a game with eight steps where Ann is the hero of a cycle over s2–s3.
fn render_hero(person: &str) -> String {
    let files = [
        (
            "game.toml",
            "[game]\ntitle = \"Koloběh\"\nlanguage = \"cs\"\nsteps = [\"Ty\", \"Potřeba\", \"Odchod\", \"Hledání\", \"Nález\", \"Cena\", \"Návrat\", \"Změna\"]\n",
        ),
        ("people/ann.toml", person),
        (
            "segments.toml",
            "[[segment]]\nid = \"s1\"\nname = \"Noc 1\"\n\n[[segment]]\nid = \"s2\"\nname = \"Noc 2\"\n\n[[segment]]\nid = \"s3\"\nname = \"Noc 3\"\n",
        ),
        (
            "players.toml",
            "[[player]]\nid = \"p1\"\nname = \"Petra\"\n",
        ),
        (
            "cycles.toml",
            "[[cycle]]\nid = \"c-ann\"\nname = \"Annin cyklus\"\nhero = \"ann\"\nfrom = \"s2\"\nto = \"s3\"\n",
        ),
        (
            "actions/talk.toml",
            "[decision]\ntitle = \"Talk\"\nowners = [\"ann\"]\nbrief = \"Speak up.\"\ncycle = \"c-ann\"\nstep = 2\n\n[[decision.option]]\nlabel = \"Yes\"\n",
        ),
    ];
    let files: Vec<SourceFile> = files
        .iter()
        .map(|(p, t)| SourceFile {
            path: (*p).to_owned(),
            text: (*t).to_owned(),
        })
        .collect();
    let checked = load(&files);
    let g = checked.game.unwrap();
    let id = Id::parse("ann").unwrap();
    let graph = larp_graph::build(&g, &[], None);
    let destiny = larp_graph::destiny(&g, &graph, &id);
    larp_render::player_sheet_html(&g, &id, &destiny, None).unwrap()
}

#[test]
fn hero_dying_at_a_step_shows_the_step_name() {
    let person = ANN.replace(
        "[person.sheet]",
        "played_by = \"p1\"\ndies = 6\n\n[person.sheet]",
    );
    let html = render_hero(&person);
    assert!(
        html.contains("Umíráš:</b> 6. Cena (Annin cyklus)"),
        "{html}"
    );
}

#[test]
fn step_phases_follow_step_order_with_their_labels() {
    // Listed out of order, and mixed with a segment phase: Noc 1 (tick 0) < step 2 < step 5.
    let person = ANN.replace(
        "[person.sheet]",
        "played_by = \"p1\"\ndies = 6\n\n[person.sheet]",
    ) + "\n[[person.phase]]\nstep = 5\ngoals = [\"Najít klíč\"]\n\n\
         [[person.phase]]\nstep = 2\ngoals = [\"Odejít z domu\"]\n\n\
         [[person.phase]]\nsegment = \"s1\"\ngoals = [\"Čekat\"]\n";
    let html = render_hero(&person);
    let first = html.find("Od: Noc 1").unwrap();
    let second = html.find("Od: 2. Potřeba (Annin cyklus)").unwrap();
    let third = html.find("Od: 5. Nález (Annin cyklus)").unwrap();
    assert!(first < second && second < third, "{html}");
    assert!(html.contains("Najít klíč"), "{html}");
    assert!(html.contains("Odejít z domu"), "{html}");
}
