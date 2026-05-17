use solito_terminal::{ScreenSnapshot, TerminalState};
use std::{
    error::Error,
    path::Path,
    sync::mpsc::{Receiver, Sender, channel},
};
use tracing::error;

use crate::session::runtime::{SessionInput, SessionRuntime};

pub(super) trait TerminalTab {
    fn input_tx(&self) -> &Sender<SessionInput>;
    fn snapshot(&self) -> ScreenSnapshot;
    fn title(&self) -> &str;
    fn drain_output(&mut self) -> bool;
    fn resize(&mut self, cols: usize, rows: usize) -> Result<(), Box<dyn Error>>;
}

pub(super) struct Tab {
    terminal: TerminalState,
    input_tx: Sender<SessionInput>,
    output_rx: Receiver<Vec<u8>>,
    title: String,
}

impl Tab {
    fn spawn(cols: usize, rows: usize, shell_program: String) -> Self {
        let (input_tx, input_rx) = channel::<SessionInput>();
        let (output_tx, output_rx) = channel::<Vec<u8>>();
        let title: String = tab_title_for_program(&shell_program);

        std::thread::spawn(move || {
            let runtime = SessionRuntime::new(input_rx, output_tx, cols, rows, shell_program);
            if let Err(err) = runtime.run_session() {
                error!("run session failed: {}", err);
            };
        });

        Self {
            terminal: TerminalState::new(cols, rows),
            input_tx,
            output_rx,
            title,
        }
    }
}

impl TerminalTab for Tab {
    fn input_tx(&self) -> &Sender<SessionInput> {
        &self.input_tx
    }

    fn snapshot(&self) -> ScreenSnapshot {
        self.terminal.snapshot()
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn drain_output(&mut self) -> bool {
        let mut updated: bool = false;
        while let Ok(output) = self.output_rx.try_recv() {
            self.terminal.apply_terminal_output(&output);
            updated = true;
        }

        updated
    }

    fn resize(&mut self, cols: usize, rows: usize) -> Result<(), Box<dyn Error>> {
        self.terminal.set_width(cols);
        self.terminal.set_height(rows);
        self.input_tx.send(SessionInput::resize(cols, rows))?;

        Ok(())
    }
}

pub(super) type AppTabs = Tabs<Tab>;

pub(super) struct Tabs<T> {
    tabs: Vec<T>,
    active: usize,
}

impl<T> Tabs<T> {
    pub(super) fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active: 0,
        }
    }

    fn push(&mut self, tab: T) {
        self.tabs.push(tab);
        self.active = self.tabs.len().saturating_sub(1);
    }

    pub(super) fn close_active(&mut self) -> bool {
        if self.tabs.is_empty() {
            return false;
        }

        self.tabs.remove(self.active);
        self.active = self.active.min(self.tabs.len().saturating_sub(1));
        true
    }

    pub(super) fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }
}

impl Tabs<Tab> {
    pub(super) fn open(&mut self, cols: usize, rows: usize, shell_program: String) {
        self.push(Tab::spawn(cols, rows, shell_program));
    }
}

impl<T: TerminalTab> Tabs<T> {
    pub(super) fn active_input_tx(&self) -> Option<&Sender<SessionInput>> {
        self.active_tab().map(TerminalTab::input_tx)
    }

    pub(super) fn active_snapshot(&self) -> Option<ScreenSnapshot> {
        self.active_tab().map(TerminalTab::snapshot)
    }

    pub(super) fn active_index(&self) -> usize {
        self.active
    }

    pub(super) fn titles(&self) -> Vec<String> {
        self.tabs
            .iter()
            .map(|tab| tab.title().to_string())
            .collect()
    }

    pub(super) fn drain_outputs(&mut self) -> bool {
        let active: usize = self.active;
        let mut active_updated: bool = false;

        for (index, tab) in self.tabs.iter_mut().enumerate() {
            let updated: bool = tab.drain_output();
            active_updated |= updated && index == active;
        }

        active_updated
    }

    pub(super) fn resize_all(&mut self, cols: usize, rows: usize) -> Result<(), Box<dyn Error>> {
        for tab in &mut self.tabs {
            tab.resize(cols, rows)?;
        }

        Ok(())
    }

    pub(super) fn activate_next(&mut self) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }

        self.active = (self.active + 1) % self.tabs.len();
        true
    }

    pub(super) fn activate_previous(&mut self) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }

        self.active = if self.active == 0 {
            self.tabs.len() - 1
        } else {
            self.active - 1
        };

        true
    }

    fn active_tab(&self) -> Option<&T> {
        self.tabs.get(self.active)
    }
}

fn tab_title_for_program(program: &str) -> String {
    let program: &str = program.trim();

    if program.is_empty() {
        return "shell".to_string();
    }

    Path::new(program)
        .file_stem()
        .or_else(|| Path::new(program).file_name())
        .and_then(|title| title.to_str())
        .filter(|title| !title.is_empty())
        .unwrap_or(program)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{Tabs, TerminalTab, tab_title_for_program};
    use crate::session::runtime::SessionInput;
    use solito_terminal::ScreenSnapshot;
    use std::{
        error::Error,
        sync::mpsc::{Receiver, Sender, channel},
    };

    struct FakeTab {
        input_tx: Sender<SessionInput>,
        _input_rx: Receiver<SessionInput>,
    }

    impl FakeTab {
        fn new() -> Self {
            let (input_tx, input_rx): (Sender<SessionInput>, Receiver<SessionInput>) =
                channel::<SessionInput>();
            Self {
                input_tx,
                _input_rx: input_rx,
            }
        }
    }

    impl TerminalTab for FakeTab {
        fn input_tx(&self) -> &Sender<SessionInput> {
            &self.input_tx
        }

        fn snapshot(&self) -> ScreenSnapshot {
            ScreenSnapshot::default()
        }

        fn title(&self) -> &str {
            "fake"
        }

        fn drain_output(&mut self) -> bool {
            false
        }

        fn resize(&mut self, _cols: usize, _rows: usize) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
    }

    #[test]
    fn activating_tabs_wraps_around() {
        let mut tabs: Tabs<FakeTab> = Tabs::new();
        tabs.push(FakeTab::new());
        tabs.push(FakeTab::new());

        assert_eq!(tabs.active, 1);
        assert!(tabs.activate_next());
        assert_eq!(tabs.active, 0);
        assert!(tabs.activate_previous());
        assert_eq!(tabs.active, 1);
    }

    #[test]
    fn closing_active_tab_selects_neighbor() {
        let mut tabs: Tabs<FakeTab> = Tabs::new();
        tabs.push(FakeTab::new());
        tabs.push(FakeTab::new());
        tabs.push(FakeTab::new());

        assert_eq!(tabs.active, 2);
        assert!(tabs.close_active());
        assert_eq!(tabs.active, 1);
        assert_eq!(tabs.titles(), vec!["fake".to_string(), "fake".to_string()]);
    }

    #[test]
    fn closing_last_tab_leaves_tabs_empty() {
        let mut tabs: Tabs<FakeTab> = Tabs::new();
        tabs.push(FakeTab::new());

        assert!(tabs.close_active());
        assert!(tabs.is_empty());
        assert_eq!(tabs.active, 0);
    }

    #[test]
    fn tab_title_uses_shell_program_name() {
        assert_eq!(tab_title_for_program("nu"), "nu");
        assert_eq!(tab_title_for_program("C:\\tools\\nu.exe"), "nu");
    }
}
