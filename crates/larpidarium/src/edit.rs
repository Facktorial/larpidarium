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
    /// Cycle id; when set, `segment` is ignored and `step` places the node.
    #[serde(default)]
    pub(crate) cycle: String,
    #[serde(default)]
    pub(crate) step: Option<u8>,
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
    /// `draft`, `review`, `done`, or empty to remove; absent (older page) keeps the file's value.
    #[serde(default)]
    pub(crate) status: Option<String>,
    /// Writer, or empty to remove; absent keeps the file's value.
    #[serde(default)]
    pub(crate) writer: Option<String>,
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
    /// The option's index in the file before the edit; `None` for a new option. Its other keys
    /// (`change`, `require`, comments) stay with it when options are removed or reordered.
    #[serde(default)]
    pub(crate) from: Option<usize>,
    /// The label the option at `from` had when the form was loaded. When the file has changed
    /// since and that option is labelled differently, the edit is refused instead of moving
    /// `change`/`require` onto the wrong option.
    #[serde(default)]
    pub(crate) from_label: Option<String>,
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
    // Switching kind would lose counter keys the form does not carry.
    if let Some(old) = doc.get(other).and_then(Item::as_table)
        && has_counter_keys(old)
    {
        return refuse(format!(
            "`{}` has `change` or `require`; remove them in the file before turning it into {}",
            edit.id,
            if key == "action" {
                "an action"
            } else {
                "a decision"
            }
        ));
    }
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
    if edit.cycle.is_empty() {
        table.remove("cycle");
        table.remove("step");
        if edit.segment.is_empty() {
            table.remove("segment");
        } else {
            set(table, "segment", Value::from(id(&edit.segment)?.as_str()));
        }
    } else {
        table.remove("segment");
        set(table, "cycle", Value::from(id(&edit.cycle)?.as_str()));
        set(
            table,
            "step",
            Value::from(i64::from(edit.step.unwrap_or(1).max(1))),
        );
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
    if let Some(status) = &edit.status {
        match status.as_str() {
            "" => {
                table.remove("status");
            }
            "draft" | "review" | "done" => set(table, "status", Value::from(status.as_str())),
            other => return refuse(format!("unknown status `{other}`")),
        }
    }
    if let Some(writer) = &edit.writer {
        if writer.trim().is_empty() {
            table.remove("writer");
        } else {
            set(table, "writer", Value::from(writer.trim()));
        }
    }
    Ok(doc.to_string())
}

/// True when a node table, or any of its options, has `change` or `require`.
fn has_counter_keys(table: &Table) -> bool {
    let own = table.contains_key("change") || table.contains_key("require");
    let in_options = match table.get("option") {
        Some(Item::ArrayOfTables(list)) => list
            .iter()
            .any(|o| o.contains_key("change") || o.contains_key("require")),
        Some(Item::Value(Value::Array(list))) => list
            .iter()
            .filter_map(Value::as_inline_table)
            .any(|o| o.contains_key("change") || o.contains_key("require")),
        _ => false,
    };
    own || in_options
}

/// Rewrites `[[decision.option]]`. An option edited from an existing one (`from`) starts from
/// that table, so its comments and keys the form does not show (`change`, `require`) follow it
/// when options are removed or reordered.
fn options(table: &mut Table, edits: &[OptionEdit]) -> EditResult<()> {
    let old: Vec<Table> = match table.get("option") {
        None => Vec::new(),
        Some(Item::ArrayOfTables(list)) => list.iter().cloned().collect(),
        Some(_) => {
            return refuse(
                "the options are not written as [[decision.option]] tables; edit this file by hand",
            );
        }
    };
    let mut used = vec![false; old.len()];
    let mut list = ArrayOfTables::new();
    for o in edits {
        require(&o.label, "option label")?;
        let mut t = match o.from {
            None => Table::new(),
            Some(i) => {
                let Some(t) = old.get(i) else {
                    return refuse("the options changed in the file since the form was opened");
                };
                let label = t.get("label").and_then(Item::as_str).unwrap_or_default();
                let moved = o.from_label.as_deref().is_some_and(|l| l != label);
                if moved || std::mem::replace(&mut used[i], true) {
                    return refuse("the options changed in the file since the form was opened");
                }
                t.clone()
            }
        };
        set(&mut t, "label", Value::from(o.label.trim()));
        set_list(&mut t, "needs", &ids(&o.needs)?);
        set_list(&mut t, "uses", &ids(&o.uses)?);
        set_list(&mut t, "gives", &ids(&o.gives)?);
        list.push(t);
    }
    table.insert("option", Item::ArrayOfTables(list));
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

/// The tile map as edited in the browser.
#[derive(Debug, Deserialize)]
pub(crate) struct MapEdit {
    /// Grid rows: `.` empty, `=` corridor.
    pub(crate) cells: Vec<String>,
    /// Walls `[x1, y1, x2, y2]` along grid lines.
    #[serde(default)]
    pub(crate) walls: Vec<[u16; 4]>,
    /// Room placements by room id; empty for a preset.
    #[serde(default)]
    pub(crate) rooms: std::collections::BTreeMap<String, MapRoomEdit>,
}

/// One room's placement, in cells.
#[derive(Debug, Deserialize)]
pub(crate) struct MapRoomEdit {
    pub(crate) x: u16,
    pub(crate) y: u16,
    pub(crate) w: u16,
    pub(crate) h: u16,
}

/// A floor plan saved under a name into `map-presets/<name>.toml`.
#[derive(Debug, Deserialize)]
pub(crate) struct PresetEdit {
    /// Preset id (kebab-case), the file name.
    pub(crate) name: String,
    /// Walls and corridors; rooms are ignored.
    #[serde(flatten)]
    pub(crate) map: MapEdit,
}

impl PresetEdit {
    /// The preset's file name, after checking the id.
    pub(crate) fn file(&self) -> EditResult<String> {
        Ok(format!("map-presets/{}.toml", id(&self.name)?))
    }
}

/// New text of `map.toml` (the whole file is written by the browser's map editor).
pub(crate) fn map(edit: &MapEdit) -> EditResult<String> {
    use std::fmt::Write as _;
    let mut out = shape(edit, "Tile map of the places")?;
    out.push_str("\n[map.rooms]\n");
    for (room, r) in &edit.rooms {
        let room = id(room)?;
        let _ = writeln!(
            out,
            "{room} = {{ x = {}, y = {}, w = {}, h = {} }}",
            r.x,
            r.y,
            r.w.max(1),
            r.h.max(1)
        );
    }
    Ok(out)
}

/// New text of a preset file: the floor plan only.
pub(crate) fn preset(edit: &PresetEdit) -> EditResult<String> {
    shape(
        &edit.map,
        &format!("Floor plan preset `{}`", id(&edit.name)?),
    )
}

/// `[map]` with `cells` and `walls`.
fn shape(edit: &MapEdit, title: &str) -> EditResult<String> {
    use std::fmt::Write as _;
    if let Some(bad) = edit
        .cells
        .iter()
        .flat_map(|r| r.chars())
        .find(|c| !['.', '='].contains(c))
    {
        return refuse(format!("unknown tile `{bad}`"));
    }
    if edit
        .walls
        .iter()
        .any(|[x1, y1, x2, y2]| (x1 != x2 && y1 != y2) || (x1 == x2 && y1 == y2))
    {
        return refuse("a wall must be a horizontal or vertical line");
    }
    let mut out = format!(
        "# {title}, saved from the larpidarium map view.\n\
         # cells: `.` empty, `=` corridor; walls: [x1, y1, x2, y2] along grid lines.\n\
         [map]\ncells = [\n"
    );
    for row in &edit.cells {
        let _ = writeln!(out, "  \"{row}\",");
    }
    out.push_str("]\nwalls = [\n");
    for [x1, y1, x2, y2] in &edit.walls {
        let _ = writeln!(out, "  [{x1}, {y1}, {x2}, {y2}],");
    }
    out.push_str("]\n");
    Ok(out)
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
            cycle: String::new(),
            step: None,
            needs: vec!["letter".into()],
            uses: Vec::new(),
            gives: vec!["secret".into()],
            brief: "Přečti si to.".into(),
            text: String::new(),
            options: Vec::new(),
            status: None,
            writer: None,
            create: true,
        }
    }

    fn option(label: &str) -> OptionEdit {
        OptionEdit {
            label: label.into(),
            needs: Vec::new(),
            uses: Vec::new(),
            gives: vec!["end".into()],
            from: None,
            from_label: None,
        }
    }

    #[test]
    fn status_and_writer_are_set_kept_and_removed() {
        let e = NodeEdit {
            status: Some("review".into()),
            writer: Some(" Jana ".into()),
            ..action("read")
        };
        let text = node(None, &e).unwrap();
        assert!(text.contains("status = \"review\""), "{text}");
        assert!(text.contains("writer = \"Jana\""), "{text}");
        // An older page sends neither field: the file keeps them.
        let kept = node(
            Some(&text),
            &NodeEdit {
                create: false,
                ..action("read")
            },
        )
        .unwrap();
        assert!(kept.contains("status = \"review\"") && kept.contains("writer = \"Jana\""));
        let removed = node(
            Some(&text),
            &NodeEdit {
                create: false,
                status: Some(String::new()),
                writer: Some("  ".into()),
                ..action("read")
            },
        )
        .unwrap();
        assert!(
            !removed.contains("status") && !removed.contains("writer"),
            "{removed}"
        );
        let bad = NodeEdit {
            status: Some("finished".into()),
            ..action("read")
        };
        assert!(node(None, &bad).is_err());
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
    fn cycle_replaces_segment() {
        let mut e = action("go");
        e.segment = "act-one".into();
        let with_segment = node(None, &e).unwrap();
        e.create = false;
        e.cycle = "c-ana".into();
        e.step = Some(4);
        let text = node(Some(&with_segment), &e).unwrap();
        assert!(text.contains("cycle = \"c-ana\""), "{text}");
        assert!(text.contains("step = 4"), "{text}");
        assert!(!text.contains("segment"), "{text}");
        e.cycle.clear();
        let back = node(Some(&text), &e).unwrap();
        assert!(!back.contains("cycle") && !back.contains("step"), "{back}");
        assert!(back.contains("segment = \"act-one\""), "{back}");
    }

    #[test]
    fn decision_options_keep_their_comments_and_shrink() {
        let current = "[decision]\ntitle = \"D\"\n\n# first option\n[[decision.option]]\nlabel = \"A\"\n\n[[decision.option]]\nlabel = \"B\"\n";
        let mut e = action("d");
        e.kind = NodeKind::Decision;
        e.create = false;
        e.options = vec![OptionEdit {
            from: Some(0),
            ..option("A2")
        }];
        let text = node(Some(current), &e).unwrap();
        assert!(
            text.contains("# first option\n[[decision.option]]\nlabel = \"A2\""),
            "{text}"
        );
        assert!(!text.contains("label = \"B\""));
    }

    #[test]
    fn removing_an_option_keeps_counter_keys_with_their_own_options() {
        let current = "[decision]\ntitle = \"D\"\n\n[[decision.option]]\nlabel = \"A\"\nrequire = { s = { at_least = 6 } }\n\n[[decision.option]]\nlabel = \"B\"\n\n[[decision.option]]\nlabel = \"C\"\nchange = { s = -2 }\n";
        let mut e = action("d");
        e.kind = NodeKind::Decision;
        e.create = false;
        e.options = vec![
            OptionEdit {
                from: Some(0),
                ..option("A")
            },
            OptionEdit {
                from: Some(2),
                ..option("C")
            },
            option("New"),
        ];
        let text = node(Some(current), &e).unwrap();
        let doc: toml::Value = toml::from_str(&text).unwrap();
        let opts = doc["decision"]["option"].as_array().unwrap();
        assert_eq!(opts.len(), 3, "{text}");
        assert_eq!(opts[0]["label"].as_str(), Some("A"));
        assert!(opts[0].get("require").is_some(), "{text}");
        assert_eq!(opts[1]["label"].as_str(), Some("C"));
        assert_eq!(opts[1]["change"]["s"].as_integer(), Some(-2), "{text}");
        assert!(opts[1].get("require").is_none(), "{text}");
        assert!(opts[2].get("change").is_none() && opts[2].get("require").is_none());
    }

    fn decision_edit(options: Vec<OptionEdit>) -> NodeEdit {
        let mut e = action("d");
        e.kind = NodeKind::Decision;
        e.create = false;
        e.options = options;
        e
    }

    const TWO_OPTIONS: &str = "[decision]\ntitle = \"D\"\n\n[[decision.option]]\nlabel = \"A\"\nchange = { s = 1 }\n\n[[decision.option]]\nlabel = \"B\"\n";

    #[test]
    fn option_whose_label_changed_in_the_file_is_refused() {
        let e = decision_edit(vec![OptionEdit {
            from: Some(0),
            from_label: Some("B".into()),
            ..option("B")
        }]);
        assert!(node(Some(TWO_OPTIONS), &e).is_err());
        let ok = decision_edit(vec![OptionEdit {
            from: Some(0),
            from_label: Some("A".into()),
            ..option("A2")
        }]);
        assert!(
            node(Some(TWO_OPTIONS), &ok)
                .unwrap()
                .contains("change = { s = 1 }")
        );
    }

    #[test]
    fn option_from_out_of_range_or_twice_is_refused() {
        let far = decision_edit(vec![OptionEdit {
            from: Some(2),
            ..option("X")
        }]);
        assert!(node(Some(TWO_OPTIONS), &far).is_err());
        let twice = decision_edit(vec![
            OptionEdit {
                from: Some(0),
                ..option("A")
            },
            OptionEdit {
                from: Some(0),
                ..option("A again")
            },
        ]);
        assert!(node(Some(TWO_OPTIONS), &twice).is_err());
    }

    #[test]
    fn inline_option_array_is_refused_not_dropped() {
        let current =
            "[decision]\ntitle = \"D\"\noption = [{ label = \"A\", change = { s = 1 } }]\n";
        let e = decision_edit(vec![option("A")]);
        assert!(node(Some(current), &e).is_err());
    }

    #[test]
    fn switching_kind_with_inline_option_counter_keys_is_refused() {
        let current =
            "[decision]\ntitle = \"D\"\noption = [{ label = \"A\", change = { s = 1 } }]\n";
        let mut e = action("d");
        e.create = false;
        assert!(node(Some(current), &e).is_err());
    }

    #[test]
    fn switching_kind_with_counter_keys_is_refused() {
        let current = "[action]\ntitle = \"A\"\nchange = { s = 1 }\n";
        let mut e = action("a");
        e.kind = NodeKind::Decision;
        e.create = false;
        e.options = vec![option("X")];
        assert!(node(Some(current), &e).is_err());
        let current = "[decision]\ntitle = \"D\"\n[[decision.option]]\nlabel = \"X\"\nrequire = { s = { at_most = 1 } }\n";
        let mut e = action("d");
        e.create = false;
        assert!(node(Some(current), &e).is_err());
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

    #[test]
    fn map_writes_rooms_and_refuses_bad_ids() {
        let ok: MapEdit = serde_json::from_str(
            r#"{"cells":["=."],"walls":[[0,0,3,0]],"rooms":{"bar":{"x":1,"y":2,"w":0,"h":3}}}"#,
        )
        .unwrap();
        let text = map(&ok).unwrap();
        assert!(text.contains("  \"=.\","), "{text}");
        assert!(text.contains("  [0, 0, 3, 0],"), "{text}");
        assert!(
            text.contains("bar = { x = 1, y = 2, w = 1, h = 3 }"),
            "{text}"
        );
        let bad: MapEdit =
            serde_json::from_str(r#"{"cells":[],"rooms":{"a\" = 1 #":{"x":0,"y":0,"w":1,"h":1}}}"#)
                .unwrap();
        assert!(map(&bad).is_err());
        let diagonal: MapEdit =
            serde_json::from_str(r#"{"cells":[],"walls":[[0,0,1,1]]}"#).unwrap();
        assert!(map(&diagonal).is_err());
    }

    #[test]
    fn preset_has_shape_but_no_rooms() {
        let p: PresetEdit = serde_json::from_str(
            r#"{"name":"sal-a","cells":["=="],"walls":[[0,1,2,1]],"rooms":{"bar":{"x":0,"y":0,"w":1,"h":1}}}"#,
        )
        .unwrap();
        assert_eq!(p.file().unwrap(), "map-presets/sal-a.toml");
        let text = preset(&p).unwrap();
        assert!(
            text.contains("[0, 1, 2, 1]") && !text.contains("bar"),
            "{text}"
        );
        let bad: PresetEdit = serde_json::from_str(r#"{"name":"../x","cells":[]}"#).unwrap();
        assert!(bad.file().is_err());
    }
}
