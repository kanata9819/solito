#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TabBarSnapshot {
    titles: Vec<String>,
    active_index: usize,
}

impl TabBarSnapshot {
    pub fn new(titles: Vec<String>, active_index: usize) -> Self {
        let active_index: usize = active_index.min(titles.len().saturating_sub(1));

        Self {
            titles,
            active_index,
        }
    }

    pub(crate) fn titles(&self) -> &[String] {
        &self.titles
    }

    pub(crate) fn active_index(&self) -> usize {
        self.active_index
    }
}

#[cfg(test)]
mod tests {
    use super::TabBarSnapshot;

    #[test]
    fn clamps_active_index_to_existing_tabs() {
        let snapshot: TabBarSnapshot = TabBarSnapshot::new(vec!["Tab 1".to_string()], 99);

        assert_eq!(snapshot.active_index(), 0);
    }
}
