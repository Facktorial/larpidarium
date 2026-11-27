//! Organizer sheets as HTML and LaTeX.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used)]

use larp_graph::Schedule;
use larp_model::{Game, Id, SourceFile, load};
use larp_render::{RenderError, organizer_sheet_html, organizer_sheets_tex};

/// Petra (organizer, colored) runs a decision and an action with Tom and the crypt (no color).
/// Names and texts are full of characters that need escaping.
fn game() -> Game {
    let files = [
        (
            "game.toml",
            "[game]\ntitle = \"Opatství\"\nlanguage = \"cs\"\n",
        ),
        (
            "segments.toml",
            "[[segment]]\nid = \"one\"\nname = \"Příjezd\"\n\n[[segment]]\nid = \"two\"\nname = \"Noc_2 & ráno\"\n",
        ),
        (
            "people/petra.toml",
            "[person]\nname = \"Petra <b>Řehořová</b>_#1\"\nrole = \"organizer\"\ncolor = \"#c0392b\"\n",
        ),
        (
            "people/tom.toml",
            "[person]\nname = \"Tom & <Jerry> 50 %\"\nrole = \"npc\"\n",
        ),
        (
            "rooms.toml",
            "[[room]]\nid = \"crypt\"\nname = \"Krypta\"\n",
        ),
        (
            "items.toml",
            "[[item]]\nid = \"lamp\"\nname = \"Lampa {stará}\"\ntext = \"Svítí <slabě>.\"\nowners = [\"petra\"]\nstarting = true\n",
        ),
        (
            "info.toml",
            "[[info]]\nid = \"end\"\nname = \"Konec\"\nending = true\n",
        ),
        (
            "actions/a-arrive.toml",
            "[action]\ntitle = \"Příjezd hráčů\"\nsegment = \"one\"\nowners = [\"petra\"]\ntext = \"Přivítej hráče u brány.\"\n",
        ),
        (
            "actions/b-ghost.toml",
            "[action]\ntitle = \"Duch v kryptě\"\nsegment = \"two\"\nowners = [\"petra\", \"tom\", \"crypt\"]\ntext = \"Zhasni <script>alert(1)</script> & čekej $5.\"\nbrief = \"Slyšíte kroky.\"\nneeds = [\"lamp\"]\n",
        ),
        (
            "actions/c-verdict.toml",
            "[decision]\ntitle = \"Verdikt\"\nsegment = \"two\"\nowners = [\"petra\"]\n\n[[decision.option]]\nlabel = \"Obvinit\"\ngives = [\"end\"]\n",
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
    assert!(checked.game.is_some(), "{:?}", checked.diagnostics);
    checked.game.unwrap()
}

fn sheet(g: &Game, owner: &str) -> Schedule {
    larp_graph::schedule(g, &Id::parse(owner).unwrap())
}

fn html(owner: &str) -> String {
    let g = game();
    organizer_sheet_html(&g, &sheet(&g, owner), None).unwrap()
}

fn tex(owners: &[&str]) -> String {
    let g = game();
    let schedules: Vec<Schedule> = owners.iter().map(|o| sheet(&g, o)).collect();
    let refs: Vec<&Schedule> = schedules.iter().collect();
    organizer_sheets_tex(&g, &refs, None).unwrap()
}

// ---------- HTML ----------

#[test]
fn html_shows_name_load_table_prepare_and_steps() {
    let out = html("petra");
    assert!(
        out.contains("Petra &lt;b&gt;Řehořová&lt;&#x2f;b&gt;_#1"),
        "{out}"
    );
    assert!(out.contains("Vytížení"));
    assert!(out.contains(r#"<table class="load">"#));
    assert!(out.contains("<th>Příjezd</th>"));
    assert!(
        out.contains(r#"<td class="peak">2</td>"#),
        "the second segment is the busiest"
    );
    assert!(out.contains("<td>1</td>"));
    assert!(out.contains("Připrav si"));
    assert!(out.contains("<b>Lampa {stará}</b>"));
    assert!(out.contains("Příjezd hráčů"));
    assert!(
        out.contains("Přivítej hráče u brány."),
        "the organizer `text`"
    );
    assert!(out.contains("Slyšíte kroky."), "the player `brief`");
    assert!(out.contains("Rozhodnutí"));
    assert!(out.contains("Obvinit"));
}

#[test]
fn html_shows_the_room_label_and_no_prepare_for_a_room() {
    let out = html("crypt");
    assert!(out.contains("<h1>Krypta</h1>"));
    assert!(out.contains("místnost"));
    assert!(
        !out.contains("Připrav si"),
        "the crypt has nothing to prepare"
    );
}

#[test]
fn html_escapes_user_text() {
    let out = html("petra");
    assert!(!out.contains("<script>"), "{out}");
    assert!(out.contains("Zhasni &lt;script&gt;alert(1)&lt;&#x2f;script&gt; &amp; čekej $5."));
    assert!(!out.contains("<b>Řehořová</b>"));
    assert!(out.contains("Svítí &lt;slabě&gt;."));
}

#[test]
fn html_escapes_the_names_of_the_other_owners() {
    let out = html("petra");
    assert!(
        out.contains("Tom &amp; &lt;Jerry&gt; 50 %, Krypta"),
        "{out}"
    );
    assert!(!out.contains("<Jerry>"));
}

#[test]
fn html_accent_falls_back_without_a_color() {
    assert!(html("petra").contains("--accent: #c0392b;"));
    assert!(html("crypt").contains("--accent: #3e5c7a;"));
}

#[test]
fn html_custom_template_is_used_and_escaped() {
    let g = game();
    let out = organizer_sheet_html(
        &g,
        &sheet(&g, "tom"),
        Some("[{{ s.name }}|{{ s.load.total }}]"),
    )
    .unwrap();
    assert_eq!(out, "[Tom &amp; &lt;Jerry&gt; 50 %|1]");
}

#[test]
fn html_invalid_template_is_a_template_error() {
    let g = game();
    let err = organizer_sheet_html(&g, &sheet(&g, "tom"), Some("{% for %}")).unwrap_err();
    assert!(matches!(err, RenderError::Template(_)), "{err:?}");
}

// ---------- LaTeX ----------

#[test]
fn tex_document_has_preamble_name_and_steps() {
    let out = tex(&["petra"]);
    assert!(out.starts_with(r"\documentclass"), "{out}");
    assert!(out.contains(r"\usepackage[czech]{babel}"));
    assert!(out.contains(r"\orgpart{Vytížení}"));
    assert!(out.contains(r"\orgpart{Připrav si}"));
    assert!(out.contains("Přivítej hráče u brány."));
    assert!(
        out.contains(r"\cellcolor{accent!25}\textbf{2}"),
        "the busiest segment"
    );
    assert!(out.trim_end().ends_with(r"\end{document}"));
    assert!(
        !out.contains(r"\clearpage"),
        "one sheet needs no page reset"
    );
}

#[test]
fn tex_escapes_special_characters() {
    let out = tex(&["petra"]);
    assert!(out.contains(r"Petra <b>Řehořová</b>\_\#1"), "{out}");
    assert!(out.contains(r"Zhasni <script>alert(1)</script> \& čekej \$5."));
    assert!(
        out.contains(r"Noc\_2 \& ráno"),
        "segment names in the load table"
    );
    assert!(out.contains(r"{\bfseries Lampa \{stará\}}"));
    assert!(
        out.contains(r"Tom \& <Jerry> 50 \%, Krypta"),
        "the other owners"
    );
    assert!(!out.contains("Tom & "));
    assert!(!out.contains("50 %"));
}

#[test]
fn tex_several_schedules_share_one_document_with_page_resets_between() {
    let out = tex(&["crypt", "petra", "tom"]);
    assert_eq!(out.matches(r"\documentclass").count(), 1);
    assert_eq!(out.matches(r"\begin{document}").count(), 1);
    assert_eq!(out.matches(r"\setcounter{page}{1}").count(), 2);
    assert_eq!(out.matches(r"\orgpart{Vytížení}").count(), 3);
    let crypt = out.find(r"\selectfont\bfseries Krypta").unwrap();
    let tom = out.find(r"\selectfont\bfseries Tom").unwrap();
    assert!(crypt < tom, "sheets keep the given order");
}

#[test]
fn tex_accent_falls_back_without_a_color() {
    let out = tex(&["petra", "crypt"]);
    assert!(out.contains(r"\definecolor{accent}{HTML}{C0392B}"));
    assert!(out.contains(r"\definecolor{accent}{HTML}{3E5C7A}"));
}

#[test]
fn tex_custom_template_is_used_and_escaped() {
    let g = game();
    let s = sheet(&g, "tom");
    let tpl = "{% for x in sheets %}({{ x.schedule.name }}:{{ x.accent }}){% endfor %}";
    let out = organizer_sheets_tex(&g, &[&s], Some(tpl)).unwrap();
    assert_eq!(out, r"(Tom \& <Jerry> 50 \%:3E5C7A)");
}

#[test]
fn tex_invalid_template_is_a_template_error() {
    let g = game();
    let s = sheet(&g, "tom");
    let err = organizer_sheets_tex(&g, &[&s], Some("{% if %}")).unwrap_err();
    assert!(matches!(err, RenderError::Template(_)), "{err:?}");
}

#[test]
fn tex_accent_falls_back_for_a_color_that_is_not_six_hex_digits() {
    let g = game();
    for color in ["#gggggg", "#12345", "#1234567", "c0392b"] {
        let mut s = sheet(&g, "petra");
        s.color = color.to_owned();
        let out = organizer_sheets_tex(&g, &[&s], None).unwrap();
        assert!(
            out.contains(r"\definecolor{accent}{HTML}{3E5C7A}"),
            "{color}: {out}"
        );
    }
}
