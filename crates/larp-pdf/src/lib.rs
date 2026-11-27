#![doc = include_str!("../README.md")]

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;

/// How to install tectonic, shown when it is missing.
pub const INSTALL_HINT: &str = "install tectonic (https://tectonic-typesetting.github.io): \
    download the binary from its GitHub releases, `cargo binstall tectonic`, \
    or your package manager; then run the command again";

/// Why a PDF could not be made.
#[derive(Debug, thiserror::Error)]
pub enum PdfError {
    /// The `tectonic` program is not on `PATH`.
    #[error("`tectonic` was not found; {INSTALL_HINT}")]
    Missing,
    /// tectonic ran but failed; `log` is its output.
    #[error("tectonic could not compile {file}:\n{log}")]
    Failed {
        /// The `.tex` file.
        file: PathBuf,
        /// What tectonic printed (the end of it).
        log: String,
    },
    /// Running tectonic or removing an old PDF failed.
    #[error("cannot run tectonic on {file}: {source}")]
    Io {
        /// The `.tex` file.
        file: PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
}

/// Compiles `tex` (a `.tex` file) with tectonic into a PDF next to it and returns the PDF path.
/// The first run downloads the LaTeX packages tectonic needs; later runs use its cache.
///
/// # Errors
///
/// [`PdfError::Missing`] without tectonic, [`PdfError::Failed`] with its log when LaTeX fails.
pub fn compile(tex: &Path) -> Result<Compiled, PdfError> {
    let dir = tex
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let pdf = tex.with_extension("pdf");
    // A PDF left from an earlier run would look current after a failed compile.
    match std::fs::remove_file(&pdf) {
        Err(e) if e.kind() != ErrorKind::NotFound => {
            return Err(PdfError::Io {
                file: tex.to_owned(),
                source: e,
            });
        }
        _ => {}
    }
    let output = Command::new("tectonic")
        .arg("--chatter")
        .arg("minimal")
        .arg("--outdir")
        .arg(dir)
        .arg(tex)
        .output()
        .map_err(|source| match source.kind() {
            ErrorKind::NotFound => PdfError::Missing,
            _ => PdfError::Io {
                file: tex.to_owned(),
                source,
            },
        })?;
    let mut log = String::from_utf8_lossy(&output.stdout).into_owned();
    log.push_str(&String::from_utf8_lossy(&output.stderr));
    if !output.status.success() {
        return Err(PdfError::Failed {
            file: tex.to_owned(),
            log: tail(&log, 40),
        });
    }
    Ok(Compiled {
        missing_glyphs: missing_glyphs(&log),
        pdf,
    })
}

/// A compiled PDF and what went missing on the way.
#[derive(Debug)]
pub struct Compiled {
    /// The PDF file.
    pub pdf: PathBuf,
    /// Characters the fonts have no glyph for (they are left out of the PDF), deduplicated.
    pub missing_glyphs: Vec<String>,
}

/// `Missing character: There is no X (U+…) in font …` lines, reduced to the characters.
fn missing_glyphs(log: &str) -> Vec<String> {
    let mut found: Vec<String> = log
        .lines()
        .filter_map(|l| {
            l.split_once("Missing character: There is no ")?
                .1
                .split(" in font")
                .next()
        })
        .map(str::to_owned)
        .collect();
    found.sort();
    found.dedup();
    found
}

/// The last `lines` lines of `text`.
fn tail(text: &str, lines: usize) -> String {
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

/// Whether tectonic can be started.
#[must_use]
pub fn available() -> bool {
    Command::new("tectonic")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_keeps_the_last_lines() {
        assert_eq!(tail("a\nb\nc", 2), "b\nc");
        assert_eq!(tail("a", 5), "a");
        assert_eq!(tail("", 5), "");
    }

    #[test]
    fn missing_glyphs_are_collected_once() {
        let log = "x\nMissing character: There is no 😀 (U+1F600) in font lmroman10-regular!\n\
                   Missing character: There is no 😀 (U+1F600) in font lmroman10-bold!\nok";
        assert_eq!(missing_glyphs(log), vec!["😀 (U+1F600)".to_owned()]);
        assert_eq!(missing_glyphs("all fine"), Vec::<String>::new());
    }

    #[test]
    fn missing_tex_file_is_an_error_not_a_panic() {
        if !available() {
            return; // Without tectonic there is nothing to run.
        }
        let r = compile(Path::new("/nonexistent/larpidarium/x.tex"));
        assert!(matches!(
            r,
            Err(PdfError::Failed { .. } | PdfError::Io { .. })
        ));
    }
}
