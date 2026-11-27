//! Turning edits from the browser into changes of TOML files.
//!
//! Every function takes the current file text (or `None` for a new file) and returns the new text.
//! Files are changed with `toml_edit`, so comments and formatting of untouched keys survive, and
//! comments in front of a changed key or after its value are kept as well.

use larp_model::Id;
use serde::Deserialize;
use toml_edit::{Array, ArrayOfTables, DocumentMut, Item, Table, Value};

/// Why an edit was refused.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct EditError(pub(crate) String);

impl std::fmt::Display for EditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

type EditResult<T> = Result<T, EditError>;

fn refuse<T>(msg: impl Into<String>) -> EditResult<T> {
    Err(EditError(msg.into()))
}

/// Action or decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum NodeKind {
    /// `[action]`.
    Action,
    /// `[decision]`.
    Decision,
}

/// A whole action or decision as edited in the browser.
#[derive(Debug, Deserialize)]
pub(crate) struct NodeEdit {
    pub(crate) id: String,
    pub(crate) kind: NodeKind,
    pub(crate) title: String,
    #[serde(default)]
    pub(crate) owners: Vec<String>,
    #[serde(default)]
    pub(crate) segment: String,
    #[serde(default)]
    pub(crate) needs: Vec<String>,
    #[serde(default)]
    pub(crate) uses: Vec<String>,
    #[serde(default)]
    pub(crate) gives: Vec<String>,
    #[serde(default)]
    pub(crate) brief: String,
    #[serde(default)]
    pub(crate) text: String,
    #[serde(default)]
    pub(crate) options: Vec<OptionEdit>,
    /// `true` for a new node: refused if the id already exists.
    #[serde(default)]
    pub(crate) create: bool,
}

/// One option of a decision.
#[derive(Debug, Deserialize)]
pub(crate) struct OptionEdit {
    pub(crate) label: String,
    #[serde(default)]
    pub(crate) needs: Vec<String>,
    #[serde(default)]
    pub(crate) uses: Vec<String>,
    #[serde(default)]
    pub(crate) gives: Vec<String>,
}

/// Item or info.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ThingKind {
    /// `[[item]]` in `items.toml`.
    Item,
    /// `[[info]]` in `info.toml`.
    Info,
}

impl ThingKind {
    /// File the thing lives in.
    pub(crate) fn file(self) -> &'static str {
        match self {
            Self::Item => "items.toml",
            Self::Info => "info.toml",
        }
    }

    fn table(self) -> &'static str {
        match self {
            Self::Item => "item",
            Self::Info => "info",
        }
    }
}

/// An item or info entry.
#[derive(Debug, Deserialize)]
pub(crate) struct ThingEdit {
    pub(crate) kind: ThingKind,
    pub(crate) id: String,
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) category: String,
    #[serde(default)]
    pub(crate) owners: Vec<String>,
    #[serde(default)]
    pub(crate) starting: bool,
    #[serde(default)]
    pub(crate) ending: bool,
    #[serde(default)]
    pub(crate) text: String,
    #[serde(default)]
    pub(crate) create: bool,
}

/// A segment entry.
#[derive(Debug, Deserialize)]
pub(crate) struct SegmentEdit {
    pub(crate) id: String,
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) color: String,
    #[serde(default)]
    pub(crate) text: String,
    #[serde(default)]
    pub(crate) create: bool,
}

/// Checks that `raw` is a valid id.
pub(crate) fn id(raw: &str) -> EditResult<Id> {
    Id::parse(raw).map_err(|e| EditError(format!("invalid id `{raw}`: {e}")))
}

fn ids(raw: &[String]) -> EditResult<Vec<Id>> {
    raw.iter().map(|r| id(r)).collect()
}

fn parse(current: Option<&str>, file: &str) -> EditResult<DocumentMut> {
    current.unwrap_or("").parse::<DocumentMut>().map_err(|e| {
        EditError(format!(
            "{file} is not valid TOML, fix it by hand first: {e}"
        ))
    })
}

/// Replaces the value of `key`, keeping the comment after the old value.
/// Assigning through `get_mut` keeps the key, and with it the comment above the line.
fn set(table: &mut Table, key: &str, mut value: Value) {
    match table.get_mut(key) {
        Some(item) => {
            if let Some(old) = item.as_value() {
                *value.decor_mut() = old.decor().clone();
            }
            *item = Item::Value(value);
        }
        None => {
            table.insert(key, Item::Value(value));
        }
    }
}

fn set_text(table: &mut Table, key: &str, text: &str) {
    let text = text.trim_end();
    if text.is_empty() {
        table.remove(key);
    } else {
        set(table, key, Value::from(text));
    }
}

fn set_list(table: &mut Table, key: &str, list: &[Id]) {
    if list.is_empty() {
        table.remove(key);
    } else {
        set(
            table,
            key,
            Value::Array(list.iter().map(Id::as_str).collect::<Array>()),
        );
    }
}

fn set_flag(table: &mut Table, key: &str, on: bool) {
    if on {
        set(table, key, Value::from(true));
    } else {
        table.remove(key);
    }
}

fn require(text: &str, what: &str) -> EditResult<()> {
    if text.trim().is_empty() {
        refuse(format!("{what} must not be empty"))
    } else {
        Ok(())
    }
}

/// New text of `actions/<id>.toml`.
pub(crate) fn node(current: Option<&str>, edit: &NodeEdit) -> EditResult<String> {
    id(&edit.id)?;
    require(&edit.title, "title")?;
    if edit.create && current.is_some() {
        return refuse(format!("`{}` already exists", edit.id));
    }
    let mut doc = parse(current, "the action file")?;
    let (key, other) = match edit.kind {
        NodeKind::Action => ("action", "decision"),
        NodeKind::Decision => ("decision", "action"),
    };
    // Switching kind: the new table inherits the old one's comment and position.
    let old_decor = doc
        .remove(other)
        .and_then(|item| item.as_table().map(|t| t.decor().clone()));
    if !doc.contains_table(key) {
        let mut table = Table::new();
        if let Some(decor) = old_decor {
            *table.decor_mut() = decor;
        }
        doc.insert(key, Item::Table(table));
    }
    let Some(table) = doc.get_mut(key).and_then(Item::as_table_mut) else {
        return refuse(format!("`{key}` in the action file is not a table"));
    };

    set(table, "title", Value::from(edit.title.trim()));
    set_list(table, "owners", &ids(&edit.owners)?);
    if edit.segment.is_empty() {
        table.remove("segment");
    } else {
        set(table, "segment", Value::from(id(&edit.segment)?.as_str()));
    }
    match edit.kind {
        NodeKind::Action => {
            set_list(table, "needs", &ids(&edit.needs)?);
            set_list(table, "uses", &ids(&edit.uses)?);
            set_list(table, "gives", &ids(&edit.gives)?);
            table.remove("option");
        }
        NodeKind::Decision => {
            if edit.options.is_empty() {
                return refuse("a decision needs at least one option");
            }
            for key in ["needs", "uses", "gives"] {
                table.remove(key);
            }
            options(table, &edit.options)?;
        }
    }
    set_text(table, "brief", &edit.brief);
    set_text(table, "text", &edit.text);
    Ok(doc.to_string())
}

/// Updates `[[decision.option]]` in place, so comments on existing options survive.
fn options(table: &mut Table, edits: &[OptionEdit]) -> EditResult<()> {
    if !matches!(table.get("option"), Some(Item::ArrayOfTables(_))) {
        table.insert("option", Item::ArrayOfTables(ArrayOfTables::new()));
    }
    let Some(list) = table
        .get_mut("option")
        .and_then(Item::as_array_of_tables_mut)
    else {
        return refuse("`option` is not a list of tables");
    };
    while list.len() > edits.len() {
        list.remove(list.len() - 1);
    }
    for (i, o) in edits.iter().enumerate() {
        require(&o.label, "option label")?;
        if list.get(i).is_none() {
            list.push(Table::new());
        }
        let Some(t) = list.get_mut(i) else {
            return refuse("cannot add an option");
        };
        set(t, "label", Value::from(o.label.trim()));
        set_list(t, "needs", &ids(&o.needs)?);
        set_list(t, "uses", &ids(&o.uses)?);
        set_list(t, "gives", &ids(&o.gives)?);
    }
    Ok(())
}

/// Finds the entry with `id` in the array of tables `name`, creating the array if needed.
fn entry<'a>(
    doc: &'a mut DocumentMut,
    name: &str,
    id_value: &str,
    create: bool,
) -> EditResult<&'a mut Table> {
    if doc.get(name).is_none() {
        doc.insert(name, Item::ArrayOfTables(ArrayOfTables::new()));
    }
    let Some(list) = doc.get_mut(name).and_then(Item::as_array_of_tables_mut) else {
        return refuse(format!("`{name}` is not a list of [[{name}]] tables"));
    };
    let found = list
        .iter()
        .position(|t| t.get("id").and_then(Item::as_str) == Some(id_value));
    match (found, create) {
        (Some(_), true) => refuse(format!("`{id_value}` already exists")),
        (None, false) => refuse(format!("`{id_value}` not found")),
        (Some(i), false) => list
            .get_mut(i)
            .ok_or_else(|| EditError("entry vanished".into())),
        (None, true) => {
            let mut t = Table::new();
            t.insert("id", Item::Value(Value::from(id_value)));
            list.push(t);
            let index = list.len() - 1;
            list.get_mut(index)
                .ok_or_else(|| EditError("cannot add an entry".into()))
        }
    }
}

/// New text of `items.toml` or `info.toml`.
pub(crate) fn thing(current: Option<&str>, edit: &ThingEdit) -> EditResult<String> {
    let thing_id = id(&edit.id)?;
    require(&edit.name, "name")?;
    let mut doc = parse(current, edit.kind.file())?;
    let t = entry(&mut doc, edit.kind.table(), thing_id.as_str(), edit.create)?;
    set(t, "name", Value::from(edit.name.trim()));
    if edit.category.is_empty() {
        t.remove("category");
    } else {
        set(t, "category", Value::from(id(&edit.category)?.as_str()));
    }
    set_list(t, "owners", &ids(&edit.owners)?);
    set_flag(t, "starting", edit.starting);
    set_flag(t, "ending", edit.ending);
    set_text(t, "text", &edit.text);
    Ok(doc.to_string())
}

/// New text of `segments.toml`.
pub(crate) fn segment(current: Option<&str>, edit: &SegmentEdit) -> EditResult<String> {
    let segment_id = id(&edit.id)?;
    require(&edit.name, "name")?;
    let color = edit.color.trim();
    if !color.is_empty() && !is_hex_color(color) {
        return refuse(format!("color `{color}` is not #rrggbb"));
    }
    let mut doc = parse(current, "segments.toml")?;
    let t = entry(&mut doc, "segment", segment_id.as_str(), edit.create)?;
    set(t, "name", Value::from(edit.name.trim()));
    set_text(t, "color", color);
    set_text(t, "text", &edit.text);
    Ok(doc.to_string())
}

/// New text of `layout.toml` after the node `node_id` was deleted: its position and the bends
/// of its edges are removed.
pub(crate) fn layout_without(current: &str, node_id: &Id) -> EditResult<String> {
    let mut doc = parse(Some(current), "layout.toml")?;
    if let Some(nodes) = doc.get_mut("node").and_then(Item::as_table_like_mut) {
        nodes.remove(node_id.as_str());
    }
    if let Some(knees) = doc.get_mut("knees").and_then(Item::as_table_like_mut) {
        let stale: Vec<String> = knees
            .iter()
            .map(|(k, _)| k.to_owned())
            .filter(|k| edge_touches(k, node_id.as_str()))
            .collect();
        for k in stale {
            knees.remove(&k);
        }
    }
    Ok(doc.to_string())
}

/// Edge ids look like `from[.port]>to[.port]`.
fn edge_touches(edge: &str, node: &str) -> bool {
    edge.split('>')
        .any(|end| end.split('.').next() == Some(node))
}

/// Merges a layout sent by the viewer into the current `layout.toml`: positions in `new` replace
/// or add entries (other nodes and all comments stay), and `[knees]` is replaced.
pub(crate) fn layout(current: Option<&str>, new: &str) -> EditResult<String> {
    let mut doc = parse(current, "layout.toml")?;
    let incoming = parse(Some(new), "the new layout")?;
    if let Some(nodes) = incoming.get("node").and_then(Item::as_table_like) {
        if doc.get("node").and_then(Item::as_table_like).is_none() {
            let mut table = Table::new();
            table.set_implicit(true);
            doc.insert("node", Item::Table(table));
        }
        if let Some(target) = doc.get_mut("node").and_then(Item::as_table_like_mut) {
            for (id_key, value) in nodes.iter() {
                target.insert(id_key, value.clone());
            }
        }
    }
    doc.remove("knees");
    if let Some(knees) = incoming.get("knees") {
        doc.insert("knees", knees.clone());
    }
    Ok(doc.to_string())
}

fn is_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(id: &str) -> NodeEdit {
        NodeEdit {
            id: id.into(),
            kind: NodeKind::Action,
            title: "Číst dopis".into(),
            owners: vec!["marta".into()],
            segment: String::new(),
            needs: vec!["letter".into()],
            uses: Vec::new(),
            gives: vec!["secret".into()],
            brief: "Přečti si to.".into(),
            text: String::new(),
            options: Vec::new(),
            create: true,
        }
    }

    fn option(label: &str) -> OptionEdit {
        OptionEdit {
            label: label.into(),
            needs: Vec::new(),
            uses: Vec::new(),
            gives: vec!["end".into()],
        }
    }

    #[test]
    fn new_action_has_only_filled_keys_in_order() {
        let text = node(None, &action("read")).unwrap();
        assert_eq!(
            text,
            "[action]\ntitle = \"Číst dopis\"\nowners = [\"marta\"]\nneeds = [\"letter\"]\ngives = [\"secret\"]\nbrief = \"Přečti si to.\"\n"
        );
    }

    #[test]
    fn editing_keeps_comments() {
        let current = "# Header comment\n[action]\n# about the title\ntitle = \"Old\" # trailing\nowners = [\"marta\"]\nneeds = [\"letter\"]\ngives = [\"secret\"]\n";
        let mut e = action("read");
        e.create = false;
        e.title = "New".into();
        let text = node(Some(current), &e).unwrap();
        assert!(text.starts_with("# Header comment\n[action]\n# about the title\n"));
        assert!(text.contains("title = \"New\" # trailing\n"), "{text}");
    }

    #[test]
    fn create_refuses_existing_file() {
        let err = node(Some("[action]\ntitle = \"x\"\n"), &action("read")).unwrap_err();
        assert!(err.0.contains("already exists"));
    }

    #[test]
    fn invalid_ids_are_refused() {
        assert!(node(None, &action("Not Valid")).is_err());
        let mut e = action("ok");
        e.needs = vec!["../etc".into()];
        assert!(node(None, &e).is_err());
    }

    #[test]
    fn switching_to_decision_writes_options() {
        let mut e = action("read");
        e.kind = NodeKind::Decision;
        e.options = vec![option("Yes"), option("No")];
        e.segment = "act-one".into();
        let text = node(
            Some("[action]\ntitle = \"x\"\nneeds = [\"a\"]\n"),
            &NodeEdit { create: false, ..e },
        )
        .unwrap();
        assert!(!text.contains("[action]"));
        assert!(text.contains("segment = \"act-one\""));
        assert_eq!(text.matches("[[decision.option]]").count(), 2, "{text}");
        assert!(
            !text.contains("needs = [\"letter\"]"),
            "decision-level needs are dropped"
        );
    }

    #[test]
    fn decision_options_keep_their_comments_and_shrink() {
        let current = "[decision]\ntitle = \"D\"\n\n# first option\n[[decision.option]]\nlabel = \"A\"\n\n[[decision.option]]\nlabel = \"B\"\n";
        let mut e = action("d");
        e.kind = NodeKind::Decision;
        e.create = false;
        e.options = vec![option("A2")];
        let text = node(Some(current), &e).unwrap();
        assert!(
            text.contains("# first option\n[[decision.option]]\nlabel = \"A2\""),
            "{text}"
        );
        assert!(!text.contains("label = \"B\""));
    }

    #[test]
    fn decision_without_options_is_refused() {
        let mut e = action("d");
        e.kind = NodeKind::Decision;
        assert!(node(None, &e).is_err());
    }

    fn info(id: &str, create: bool) -> ThingEdit {
        ThingEdit {
            kind: ThingKind::Info,
            id: id.into(),
            name: "Konec".into(),
            category: String::new(),
            owners: Vec::new(),
            starting: false,
            ending: true,
            text: String::new(),
            create,
        }
    }

    #[test]
    fn things_are_added_and_updated_by_id() {
        let current = "# infos\n[[info]]\nid = \"a\"\nname = \"A\" # keep\n";
        let added = thing(Some(current), &info("end", true)).unwrap();
        assert!(added.starts_with("# infos\n"));
        assert!(added.contains("[[info]]\nid = \"end\"\nname = \"Konec\"\nending = true\n"));
        let mut e = info("a", false);
        e.name = "Á".into();
        e.ending = false;
        let updated = thing(Some(&added), &e).unwrap();
        assert!(updated.contains("name = \"Á\" # keep"), "{updated}");
        assert!(
            thing(Some(&updated), &info("a", true)).is_err(),
            "duplicate"
        );
        assert!(
            thing(Some(&updated), &info("zzz", false)).is_err(),
            "missing"
        );
    }

    #[test]
    fn segments_validate_colors() {
        let mut e = SegmentEdit {
            id: "act-one".into(),
            name: "Akt I".into(),
            color: "#aabbcc".into(),
            text: String::new(),
            create: true,
        };
        let text = segment(None, &e).unwrap();
        assert_eq!(
            text,
            "[[segment]]\nid = \"act-one\"\nname = \"Akt I\"\ncolor = \"#aabbcc\"\n"
        );
        e.color = "red".into();
        assert!(segment(None, &e).is_err());
    }

    #[test]
    fn deleting_a_node_removes_its_position() {
        let current = "[node.a]\nx = 1\ny = 2\n\n[node.b]\nx = 3\ny = 4\n";
        let current = format!(
            "{current}\n[knees]\n\"a.1>b\" = [{{ x = 1, y = 1 }}]\n\"b>c\" = [{{ x = 2, y = 2 }}]\n"
        );
        let text = layout_without(&current, &id("a").unwrap()).unwrap();
        assert!(!text.contains("node.a"));
        assert!(text.contains("[node.b]"));
        assert!(
            !text.contains("a.1>b"),
            "bends of the deleted node's edges go too"
        );
        assert!(text.contains("\"b>c\""));
    }

    #[test]
    fn saving_the_layout_merges_and_keeps_comments() {
        let current = "# my notes\n[node.a]\nx = 1\ny = 2\n\n[node.b]\nx = 3\ny = 4\n\n[knees]\n\"a>b\" = []\n";
        let new = "[node.b]\nx = 30\ny = 40\n\n[node.c]\nx = 5\ny = 6\n";
        let text = layout(Some(current), new).unwrap();
        assert!(text.starts_with("# my notes\n[node.a]\nx = 1"), "{text}");
        assert!(text.contains("x = 30"));
        assert!(text.contains("[node.c]"));
        assert!(
            !text.contains("knees"),
            "knees are replaced by the new (empty) set"
        );
        assert!(layout(None, new).unwrap().contains("[node.c]"));
    }

    #[test]
    fn switching_kind_keeps_the_comment_above_the_table() {
        let mut e = action("d");
        e.kind = NodeKind::Decision;
        e.create = false;
        e.options = vec![option("A")];
        let text = node(Some("# keep me\n[action]\ntitle = \"x\"\n"), &e).unwrap();
        assert!(text.starts_with("# keep me\n[decision]"), "{text}");
    }

    #[test]
    fn broken_files_are_not_overwritten() {
        let err = thing(Some("[[info]\n"), &info("a", true)).unwrap_err();
        assert!(err.0.contains("fix it by hand"));
        assert!(layout(None, "x = ").is_err());
    }
}
