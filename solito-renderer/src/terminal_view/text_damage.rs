use solito_terminal::ScreenSnapshot;
use std::collections::BTreeSet;

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) enum TextDamage {
    #[default]
    None,
    Rows(BTreeSet<usize>),
    All,
}

impl TextDamage {
    pub(super) fn between(previous: &ScreenSnapshot, next: &ScreenSnapshot) -> Self {
        let mut damage = Self::None;
        let row_count = previous.lines.len().max(next.lines.len());

        for row in 0..row_count {
            if previous.lines.get(row) != next.lines.get(row) {
                damage.add_row(row);
            }
        }

        let cursor_changed = previous.cursor_row != next.cursor_row
            || previous.cursor_col != next.cursor_col
            || previous.cursor_color != next.cursor_color
            || previous.cursor_visible != next.cursor_visible;

        if cursor_changed {
            if previous.cursor_visible {
                damage.add_row(previous.cursor_row);
            }
            if next.cursor_visible {
                damage.add_row(next.cursor_row);
            }
        }

        damage
    }

    pub(super) fn add_row(&mut self, row: usize) {
        match self {
            Self::None => {
                *self = Self::Rows(BTreeSet::from([row]));
            }
            Self::Rows(rows) => {
                rows.insert(row);
            }
            Self::All => {}
        }
    }

    pub(super) fn mark_all(&mut self) {
        *self = Self::All;
    }

    pub(super) fn merge(&mut self, next: Self) {
        match next {
            Self::None => {}
            Self::All => self.mark_all(),
            Self::Rows(next_rows) => match self {
                Self::None => *self = Self::Rows(next_rows),
                Self::Rows(rows) => rows.extend(next_rows),
                Self::All => {}
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TextDamage;
    use solito_terminal::{ScreenCell, ScreenSnapshot};
    use std::collections::BTreeSet;

    fn line(text: &str) -> Vec<ScreenCell> {
        text.chars()
            .map(|ch| {
                let mut cell = ScreenCell::default();
                cell.ch = ch;
                cell
            })
            .collect()
    }

    #[test]
    fn detects_only_changed_rows() {
        let previous = ScreenSnapshot {
            lines: vec![line("same"), line("old"), line("same")],
            cursor_visible: false,
            ..ScreenSnapshot::default()
        };
        let next = ScreenSnapshot {
            lines: vec![line("same"), line("new"), line("same")],
            cursor_visible: false,
            ..ScreenSnapshot::default()
        };

        assert_eq!(
            TextDamage::between(&previous, &next),
            TextDamage::Rows(BTreeSet::from([1]))
        );
    }

    #[test]
    fn cursor_movement_damages_old_and_new_rows() {
        let previous = ScreenSnapshot {
            lines: vec![line("a"), line("b"), line("c")],
            cursor_row: 0,
            cursor_visible: true,
            ..ScreenSnapshot::default()
        };
        let next = ScreenSnapshot {
            lines: previous.lines.clone(),
            cursor_row: 2,
            cursor_visible: true,
            ..ScreenSnapshot::default()
        };

        assert_eq!(
            TextDamage::between(&previous, &next),
            TextDamage::Rows(BTreeSet::from([0, 2]))
        );
    }

    #[test]
    fn hidden_cursor_movement_does_not_damage_text() {
        let previous = ScreenSnapshot {
            lines: vec![line("a"), line("b")],
            cursor_row: 0,
            cursor_visible: false,
            ..ScreenSnapshot::default()
        };
        let next = ScreenSnapshot {
            lines: previous.lines.clone(),
            cursor_row: 1,
            cursor_visible: false,
            ..ScreenSnapshot::default()
        };

        assert_eq!(TextDamage::between(&previous, &next), TextDamage::None);
    }

    #[test]
    fn merge_keeps_all_as_the_strongest_damage() {
        let mut damage = TextDamage::Rows(BTreeSet::from([1]));
        damage.mark_all();
        damage.merge(TextDamage::Rows(BTreeSet::from([2])));

        assert_eq!(damage, TextDamage::All);
    }
}
