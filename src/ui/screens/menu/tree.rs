//! What the menu lists: the entries, with open folders expanded, and which
//! one is selected. No firmware calls.

use alloc::vec::Vec;
use core::ptr;

use crate::config::{Entry, Kind};

pub struct Tree<'a> {
    entries: &'a [Entry],
    /// Expanded folders, compared by address.
    open: Vec<&'a Entry>,
    /// Index into [`Tree::rows`].
    pub selected: usize,
}

/// One visible line of the menu.
pub struct Row<'a> {
    pub entry: &'a Entry,
    pub depth: usize,
    pub open: bool,
}

impl<'a> Tree<'a> {
    /// Selects the entry with id `default`, opening the folders leading to it.
    pub fn new(entries: &'a [Entry], default: Option<&str>) -> Self {
        let mut tree = Self {
            entries,
            open: Vec::new(),
            selected: 0,
        };
        if let Some(default) = tree.open_default(default) {
            tree.selected = tree
                .rows()
                .iter()
                .position(|row| ptr::eq(row.entry, default))
                .unwrap_or(0);
        }
        tree
    }

    /// The entries currently shown, with open folders expanded.
    pub fn rows(&self) -> Vec<Row<'a>> {
        let mut rows = Vec::new();
        self.push_rows(self.entries, 0, &mut rows);
        rows
    }

    pub fn toggle(&mut self, folder: &'a Entry) {
        match self.open.iter().position(|&open| ptr::eq(open, folder)) {
            Some(i) => {
                self.open.swap_remove(i);
            }
            None => self.open.push(folder),
        }
    }

    /// Finds the default entry and opens the folders leading to it. When the
    /// default is a folder with a default of its own, that folder is opened
    /// and its default followed, so the result is bootable whenever the
    /// config's defaults are.
    fn open_default(&mut self, id: Option<&str>) -> Option<&'a Entry> {
        let mut entry = find(self.entries, id?, &mut self.open)?;
        while let Kind::Folder(folder) = &entry.kind {
            let Some(inner) = folder
                .default
                .as_deref()
                .and_then(|id| find(&folder.entries, id, &mut self.open))
            else {
                break;
            };
            self.open.push(entry);
            entry = inner;
        }
        Some(entry)
    }

    fn is_open(&self, entry: &Entry) -> bool {
        self.open.iter().any(|&open| ptr::eq(open, entry))
    }

    fn push_rows(&self, entries: &'a [Entry], depth: usize, rows: &mut Vec<Row<'a>>) {
        for entry in entries {
            let open = self.is_open(entry);
            rows.push(Row { entry, depth, open });
            if open && let Kind::Folder(folder) = &entry.kind {
                self.push_rows(&folder.entries, depth + 1, rows);
            }
        }
    }
}

pub fn is_folder(entry: &Entry) -> bool {
    matches!(entry.kind, Kind::Folder(_))
}

/// Finds the entry with `id` anywhere in the tree, and pushes the folders
/// that contain it onto `path`.
fn find<'a>(entries: &'a [Entry], id: &str, path: &mut Vec<&'a Entry>) -> Option<&'a Entry> {
    for entry in entries {
        if entry.id == id {
            return Some(entry);
        }
        if let Kind::Folder(folder) = &entry.kind {
            path.push(entry);
            if let Some(found) = find(&folder.entries, id, path) {
                return Some(found);
            }
            path.pop();
        }
    }
    None
}
