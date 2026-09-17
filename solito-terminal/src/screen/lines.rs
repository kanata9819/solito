use super::buffer::ScreenLine;
use std::{
    collections::VecDeque,
    ops::{Index, IndexMut},
    sync::Arc,
};

const ROWS_PER_BLOCK: usize = 64;

#[derive(Clone, Debug)]
enum LineBlock {
    Editing(Vec<ScreenLine>),
    History(Arc<Vec<ScreenLine>>),
}

impl LineBlock {
    fn lines(&self) -> &Vec<ScreenLine> {
        match self {
            Self::Editing(lines) => lines,
            Self::History(lines) => lines,
        }
    }

    fn lines_mut(&mut self) -> &mut Vec<ScreenLine> {
        if matches!(self, Self::History(_)) {
            let Self::History(lines) = std::mem::replace(self, Self::Editing(Vec::new())) else {
                unreachable!()
            };
            *self = Self::Editing(Arc::unwrap_or_clone(lines));
        }
        let Self::Editing(lines) = self else {
            unreachable!()
        };
        lines
    }

    fn freeze(&mut self) {
        if let Self::Editing(lines) = self {
            *self = Self::History(Arc::new(std::mem::take(lines)));
        }
    }
}

/// Copy-on-write blocks keep snapshots cheap even with a full scrollback.
/// Dropping history releases whole blocks; at most 63 expired rows remain in the first block.
#[derive(Clone, Debug, Default)]
pub struct ScreenLines {
    blocks: VecDeque<LineBlock>,
    head: usize,
    len: usize,
}

impl ScreenLines {
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn get(&self, row: usize) -> Option<&ScreenLine> {
        if row >= self.len {
            return None;
        }
        let slot = self.head + row;
        self.blocks
            .get(slot / ROWS_PER_BLOCK)?
            .lines()
            .get(slot % ROWS_PER_BLOCK)
    }

    pub fn get_mut(&mut self, row: usize) -> Option<&mut ScreenLine> {
        if row >= self.len {
            return None;
        }
        let slot = self.head + row;
        self.blocks[slot / ROWS_PER_BLOCK]
            .lines_mut()
            .get_mut(slot % ROWS_PER_BLOCK)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &ScreenLine> + DoubleEndedIterator {
        (0..self.len).map(|row| &self[row])
    }

    pub(super) fn push_back(&mut self, line: ScreenLine) {
        if (self.head + self.len) % ROWS_PER_BLOCK == 0 {
            if let Some(block) = self.blocks.back_mut() {
                block.freeze();
            }
            self.blocks
                .push_back(LineBlock::Editing(Vec::with_capacity(ROWS_PER_BLOCK)));
        }
        self.blocks.back_mut().unwrap().lines_mut().push(line);
        self.len += 1;
    }

    pub(super) fn discard_prefix(&mut self, count: usize) {
        assert!(count <= self.len);
        if count == self.len {
            *self = Self::default();
            return;
        }
        let head = self.head + count;
        for _ in 0..head / ROWS_PER_BLOCK {
            self.blocks.pop_front();
        }
        self.head = head % ROWS_PER_BLOCK;
        self.len -= count;
    }

    pub fn remove(&mut self, row: usize) -> ScreenLine {
        assert!(row < self.len);
        let removed = self[row].clone();
        if row == 0 {
            self.discard_prefix(1);
            return removed;
        }
        for index in row..self.len - 1 {
            self[index] = self[index + 1].clone();
        }
        let tail = self.blocks.back_mut().unwrap().lines_mut();
        tail.pop();
        if tail.is_empty() {
            self.blocks.pop_back();
        }
        self.len -= 1;
        removed
    }

    pub(super) fn insert(&mut self, row: usize, line: ScreenLine) {
        assert!(row <= self.len);
        let old_len = self.len;
        self.push_back(ScreenLine::default());
        for index in (row..old_len).rev() {
            self[index + 1] = self[index].clone();
        }
        self[row] = line;
    }
}

impl Index<usize> for ScreenLines {
    type Output = ScreenLine;

    fn index(&self, row: usize) -> &Self::Output {
        self.get(row).expect("terminal row out of bounds")
    }
}

impl IndexMut<usize> for ScreenLines {
    fn index_mut(&mut self, row: usize) -> &mut Self::Output {
        self.get_mut(row).expect("terminal row out of bounds")
    }
}

impl FromIterator<ScreenLine> for ScreenLines {
    fn from_iter<T: IntoIterator<Item = ScreenLine>>(iter: T) -> Self {
        let mut lines = Self::default();
        for line in iter {
            lines.push_back(line);
        }
        lines
    }
}

impl From<Vec<ScreenLine>> for ScreenLines {
    fn from(lines: Vec<ScreenLine>) -> Self {
        lines.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ScreenCell;

    fn line(n: usize) -> ScreenLine {
        vec![ScreenCell {
            ch: char::from_u32(65 + (n % 26) as u32).unwrap(),
            ..Default::default()
        }]
        .into()
    }

    #[test]
    fn blocks_match_vec_across_history_eviction_and_screen_edits() {
        let mut expected: Vec<_> = (0..200).map(line).collect();
        let mut actual: ScreenLines = expected.clone().into();
        let saved = actual.clone();
        let original = expected.clone();
        for step in 0..600 {
            let remove = step % 5;
            actual.discard_prefix(remove);
            expected.drain(..remove);
            for n in 0..remove + 1 {
                actual.push_back(line(step + n));
                expected.push(line(step + n));
            }
            let row = step % expected.len();
            assert_eq!(actual.remove(row), expected.remove(row));
            actual.insert(row, line(step));
            expected.insert(row, line(step));
            actual[row][0].ch = '!';
            expected[row][0].ch = '!';
            assert!(actual.iter().eq(expected.iter()));
            assert!(saved.iter().eq(original.iter()));
        }
        actual.discard_prefix(actual.len());
        actual.push_back(line(0));
        assert_eq!(actual.remove(0), line(0));
        assert!(actual.is_empty());
    }
}
