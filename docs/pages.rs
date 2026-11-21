//! Runs the Rust examples on the documentation site.
//!
//! Each page in this directory becomes the documentation of an item that exists only while
//! rustdoc collects doctests, so `cargo test --package warded-docs` compiles and runs every
//! `rust` code block on the site against the Warded source in the parent directory. Examples are
//! complete programs: the site shows them as written, so they use no hidden lines.

macro_rules! pages {
    ($($page:ident => $file:literal,)*) => {
        $(
            #[cfg(doctest)]
            #[doc = include_str!($file)]
            struct $page;
        )*

        /// Every page whose examples run, for the check that none is left out.
        #[cfg(test)]
        const PAGES: &[&str] = &[$($file),*];
    };
}

pages! {
    Index => "index.md",
    GettingStarted => "getting-started.md",
    Selection => "selection.md",
    Paths => "paths.md",
    Conditions => "conditions.md",
    TransportSecurity => "transport-security.md",
    Negotiation => "negotiation.md",
    Tower => "tower.md",
    CustomMatchers => "custom-matchers.md",
    Troubleshooting => "troubleshooting.md",
    Limits => "limits.md",
    Versions => "versions.md",
    Upgrading => "upgrading.md",
    Changelog => "changelog.md",
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, fs, path::Path};

    /// Languages a page's code blocks may use. rustdoc runs blocks marked `rust`, or with no
    /// language at all, and skips the rest, so an unlisted name such as `rs` would hide an
    /// example from these checks while the site still showed it as Rust.
    const LANGUAGES: &[&str] = &["rust", "toml", "sh", "text", "diff"];

    fn pages() -> BTreeSet<String> {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR"));
        fs::read_dir(directory)
            .expect("the documentation directory is readable")
            .map(|entry| entry.expect("a directory entry").file_name())
            .filter_map(|name| name.into_string().ok())
            .filter(|name| name.ends_with(".md"))
            .collect()
    }

    #[test]
    fn every_page_runs_its_examples() {
        let listed: BTreeSet<String> = super::PAGES.iter().map(|page| page.to_string()).collect();
        assert_eq!(pages(), listed, "list each page in the pages! macro");
    }

    #[test]
    fn code_blocks_name_a_known_language() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR"));
        for page in pages() {
            let text = fs::read_to_string(directory.join(&page)).expect("a readable page");
            let mut open = false;
            for (number, line) in text.lines().enumerate() {
                let Some(info) = line.strip_prefix("```") else {
                    assert!(
                        !line.trim_start().starts_with("```") && !line.starts_with("~~~"),
                        "{page}:{}: use unindented backtick fences",
                        number + 1,
                    );
                    continue;
                };
                if open {
                    assert!(
                        info.is_empty(),
                        "{page}:{}: a fence closes a block",
                        number + 1
                    );
                } else {
                    let language = info.split_whitespace().next().unwrap_or_default();
                    assert!(
                        LANGUAGES.contains(&language),
                        "{page}:{}: name one of {LANGUAGES:?} after the fence",
                        number + 1,
                    );
                }
                open = !open;
            }
            assert!(!open, "{page}: a code block is not closed");
        }
    }
}
