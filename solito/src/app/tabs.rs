use solito_terminal::{ScreenSnapshot, TerminalState};
use std::{
    error::Error,
    sync::mpsc::{Receiver, Sender, channel},
};
use tracing::error;

use crate::session::runtime::{SessionInput, SessionRuntime};

pub(super) trait TerminalTab {
    fn input_tx(&self) -> &Sender<SessionInput>;
    fn snapshot(&self) -> ScreenSnapshot;
    fn drain_output(&mut self) -> bool;
    fn resize(&mut self, cols: usize, rows: usize) -> Result<(), Box<dyn Error>>;
}

pub(super) struct Tab {
    terminal: TerminalState,
    input_tx: Sender<SessionInput>,
    output_rx: Receiver<Vec<u8>>,
}

impl Tab {
    fn spawn(cols: usize, rows: usize) -> Self {
        let (input_tx, input_rx): (Sender<SessionInput>, Receiver<SessionInput>) =
            channel::<SessionInput>();
        let (output_tx, output_rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel::<Vec<u8>>();

        std::thread::spawn(move || {
            let runtime: SessionRuntime = SessionRuntime::new(input_rx, output_tx, cols, rows);
            if let Err(err) = runtime.run_session() {
                error!("run session failed: {}", err);
            };
        });

        Self {
            terminal: TerminalState::new(cols, rows),
            input_tx,
            output_rx,
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
}

impl Tabs<Tab> {
    pub(super) fn open(&mut self, cols: usize, rows: usize) {
        self.push(Tab::spawn(cols, rows));
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
        (0..self.tabs.len())
            .map(|index| format!("Tab {}", index + 1))
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

#[cfg(test)]
mod tests {
    use super::{Tabs, TerminalTab};
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
}
