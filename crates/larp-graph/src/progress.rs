//! Writing progress: how many nodes and people are drafts, in review or done, per writer.

use std::collections::BTreeMap;

use larp_model::Game;
use larp_model::model::Status;
use serde::Serialize;

/// Counts of one group (the whole game or one writer).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct StatusCount {
    /// `status = "draft"`.
    pub draft: usize,
    /// `status = "review"`.
    pub review: usize,
    /// `status = "done"`.
    pub done: usize,
    /// No `status`.
    pub untracked: usize,
}

impl StatusCount {
    fn add(&mut self, status: Option<Status>) {
        match status {
            Some(Status::Draft) => self.draft += 1,
            Some(Status::Review) => self.review += 1,
            Some(Status::Done) => self.done += 1,
            None => self.untracked += 1,
        }
    }

    /// Everything counted.
    #[must_use]
    pub fn total(&self) -> usize {
        self.draft + self.review + self.done + self.untracked
    }
}

/// One part that is not done yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpenPart {
    /// `"node"` or `"person"`.
    pub kind: &'static str,
    /// Node or person id.
    pub id: String,
    /// Title or name.
    pub title: String,
    /// `"draft"` or `"review"`.
    pub status: &'static str,
    /// Writer, if set.
    pub writer: Option<String>,
}

/// Writing progress of a game.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Progress {
    /// The whole game.
    pub all: StatusCount,
    /// Per writer, by name; parts without a writer are not listed here.
    pub writers: Vec<(String, StatusCount)>,
    /// Drafts and parts in review, drafts first, then by title.
    pub open: Vec<OpenPart>,
}

impl Progress {
    /// True when no node or person has a `status`, so there is nothing to report.
    #[must_use]
    pub fn is_untracked(&self) -> bool {
        self.all.untracked == self.all.total()
    }
}

/// Writing progress of every node and person.
#[must_use]
pub fn progress(game: &Game) -> Progress {
    let parts = game
        .nodes
        .iter()
        .map(|(id, n)| ("node", id, n.title(), n.status(), n.writer()))
        .chain(
            game.people
                .iter()
                .map(|(id, p)| ("person", id, p.name.as_str(), p.status, p.writer.as_deref())),
        );
    let mut out = Progress::default();
    let mut writers: BTreeMap<String, StatusCount> = BTreeMap::new();
    for (kind, id, title, status, writer) in parts {
        out.all.add(status);
        if let Some(w) = writer {
            writers.entry(w.to_owned()).or_default().add(status);
        }
        if let Some(s @ (Status::Draft | Status::Review)) = status {
            out.open.push(OpenPart {
                kind,
                id: id.to_string(),
                title: title.to_owned(),
                status: s.as_str(),
                writer: writer.map(str::to_owned),
            });
        }
    }
    out.open.sort_by(|a, b| {
        (a.status != "draft", &a.title, &a.id).cmp(&(b.status != "draft", &b.title, &b.id))
    });
    out.writers = writers.into_iter().collect();
    out
}
