use crate::session::runtime::{SessionInput, SessionRuntime};

use anyhow::Result;
use solito_terminal::{ScreenSnapshot, TerminalSize, TerminalState};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender, channel, sync_channel},
    },
};
use tracing::error;
use winit::event_loop::EventLoopProxy;

use crate::app::event::{AppEvent, OutputNotifier};

pub(super) trait TerminalTab {
    fn input_tx(&self) -> &Sender<SessionInput>;
    fn snapshot(&self) -> ScreenSnapshot;
    fn title(&self) -> &str;
    fn drain_output(&mut self) -> bool;
    fn has_exited(&self) -> bool;
    fn resize(&mut self, size: TerminalSize) -> Result<()>;
}

pub(super) struct Tab {
    terminal: TerminalState,
    input_tx: Sender<SessionInput>,
    output_rx: Receiver<Vec<u8>>,
    title: String,
    output_notifier: OutputNotifier,
    exited: Arc<AtomicBool>,
}

impl Tab {
    fn spawn(
        size: TerminalSize,
        shell_program: String,
        event_proxy: EventLoopProxy<AppEvent>,
    ) -> Self {
        let (input_tx, input_rx) = channel::<SessionInput>();
        // At most 64 chunks of 16 KiB can wait for the UI (about 1 MiB per tab).
        let (output_tx, output_rx) = sync_channel::<Vec<u8>>(64);
        let title = tab_title_for_program(&shell_program);

        let exited = Arc::new(AtomicBool::new(false));
        let runtime_exited = exited.clone();
        let output_notifier = OutputNotifier::new(event_proxy.clone());
        let runtime_notifier = output_notifier.clone();
        std::thread::spawn(move || {
            let result =
                SessionRuntime::new(input_rx, output_tx, runtime_notifier, size, &shell_program)
                    .and_then(SessionRuntime::run_session);
            if let Err(err) = result {
                error!("terminal session failed: {err}");
            }
            // Publish completion before waking the UI, including silent exits.
            // The flag belongs to this tab, so a late event cannot close a new tab.
            runtime_exited.store(true, Ordering::Release);
            let _ = event_proxy.send_event(AppEvent::TerminalSessionExited);
        });

        Self {
            terminal: TerminalState::new(size),
            input_tx,
            output_rx,
            title,
            output_notifier,
            exited,
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

    fn has_exited(&self) -> bool {
        self.exited.load(Ordering::Acquire)
    }

    fn drain_output(&mut self) -> bool {
        self.output_notifier.begin_drain();
        let mut updated = false;
        // Bound each event so continuous output cannot monopolize the UI thread.
        for _ in 0..16 {
            let Ok(output) = self.output_rx.try_recv() else {
                return updated;
            };
            self.terminal.apply_terminal_output(&output);
            let responses = self.terminal.take_responses();
            if !responses.is_empty()
                && let Err(err) = self.input_tx.send(SessionInput::write(responses))
            {
                error!("failed to send terminal response: {err}");
            }
            updated = true;
        }
        // Continue later even if the producer has stopped after filling the queue.
        self.output_notifier.notify();

        updated
    }

    fn resize(&mut self, size: TerminalSize) -> Result<()> {
        self.terminal.resize(size);
        self.input_tx.send(SessionInput::resize(size))?;

        Ok(())
    }
}

pub(super) type AppTabs = Tabs<Tab>;

#[cfg(all(test, target_os = "windows"))]
impl AppTabs {
    pub(super) fn test_fixture(
        terminal: TerminalState,
        event_proxy: EventLoopProxy<AppEvent>,
    ) -> (Self, Receiver<SessionInput>) {
        let (input_tx, input_rx) = channel();
        let (_, output_rx) = sync_channel(1);
        let mut tabs = Self::new();
        tabs.push(Tab {
            terminal,
            input_tx,
            output_rx,
            title: "fixture".into(),
            output_notifier: OutputNotifier::new(event_proxy),
            exited: Default::default(),
        });
        (tabs, input_rx)
    }
}

pub(super) struct Tabs<T> {
    tabs: Vec<T>,
    active: usize,
}

#[derive(Default)]
pub(super) struct ClosedTabs {
    pub(super) any_closed: bool,
    pub(super) active_closed: bool,
}

impl<T> Tabs<T> {
    pub(super) fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active: 0,
        }
    }

    pub(super) fn close_active(&mut self) -> bool {
        if self.tabs.is_empty() {
            return false;
        }

        self.remove(self.active);
        true
    }

    fn remove(&mut self, index: usize) {
        self.tabs.remove(index);
        if index < self.active {
            self.active -= 1;
        } else {
            self.active = self.active.min(self.tabs.len().saturating_sub(1));
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    pub(super) fn activate(&mut self, index: usize) -> bool {
        if index >= self.tabs.len() || index == self.active {
            return false;
        }
        self.active = index;
        true
    }

    fn push(&mut self, tab: T) {
        self.tabs.push(tab);
        self.active = self.tabs.len().saturating_sub(1);
    }
}

impl Tabs<Tab> {
    pub(super) fn active_mouse_mode(&self) -> solito_terminal::MouseMode {
        self.active_tab()
            .map(|tab| tab.terminal.mouse_mode())
            .unwrap_or_default()
    }

    pub(super) fn open(
        &mut self,
        size: TerminalSize,
        shell_program: String,
        event_proxy: EventLoopProxy<AppEvent>,
    ) {
        self.push(Tab::spawn(size, shell_program, event_proxy));
    }
}

impl<T: TerminalTab> Tabs<T> {
    pub(super) fn close_exited(&mut self) -> ClosedTabs {
        let mut closed = ClosedTabs::default();
        // Remove back to front so earlier indexes remain valid.
        for index in (0..self.tabs.len()).rev() {
            if self.tabs[index].has_exited() {
                closed.any_closed = true;
                closed.active_closed |= index == self.active;
                self.remove(index);
            }
        }
        closed
    }

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
        let active = self.active;
        let mut active_updated = false;

        for (index, tab) in self.tabs.iter_mut().enumerate() {
            let updated = tab.drain_output();
            active_updated |= updated && index == active;
        }

        active_updated
    }

    pub(super) fn resize_all(&mut self, size: TerminalSize) -> Result<()> {
        for tab in &mut self.tabs {
            tab.resize(size)?;
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
    let program = program.trim();

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
    use anyhow::Result;
    use solito_terminal::{ScreenSnapshot, TerminalSize};
    use std::sync::mpsc::{Receiver, Sender, channel};

    struct FakeTab {
        input_tx: Sender<SessionInput>,
        _input_rx: Receiver<SessionInput>,
        title: &'static str,
        exited: bool,
    }

    impl FakeTab {
        fn new() -> Self {
            let (input_tx, input_rx): (Sender<SessionInput>, Receiver<SessionInput>) =
                channel::<SessionInput>();
            Self {
                input_tx,
                _input_rx: input_rx,
                title: "fake",
                exited: false,
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
            self.title
        }

        fn has_exited(&self) -> bool {
            self.exited
        }

        fn drain_output(&mut self) -> bool {
            false
        }

        fn resize(&mut self, _size: TerminalSize) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn activating_tabs_wraps_around() {
        let mut tabs = Tabs::new();
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
        let mut tabs = Tabs::new();
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
        let mut tabs = Tabs::new();
        tabs.push(FakeTab::new());

        assert!(tabs.close_active());
        assert!(tabs.is_empty());
        assert_eq!(tabs.active, 0);
    }

    fn three_tabs() -> Tabs<FakeTab> {
        let mut tabs = Tabs::new();
        for title in ["first", "second", "third"] {
            tabs.push(FakeTab { title, ..FakeTab::new() });
        }
        tabs
    }

    #[test]
    fn exiting_background_tabs_preserves_the_active_session() {
        let mut tabs = three_tabs();
        tabs.activate(1);
        tabs.tabs[0].exited = true;
        tabs.tabs[2].exited = true;
        let closed = tabs.close_exited();
        assert!(closed.any_closed);
        assert!(!closed.active_closed);
        assert_eq!(tabs.active_index(), 0);
        assert_eq!(tabs.active_tab().unwrap().title(), "second");
    }

    #[test]
    fn exiting_active_tab_selects_a_surviving_neighbor() {
        let mut tabs = three_tabs();
        tabs.activate(1);
        tabs.tabs[1].exited = true;
        let closed = tabs.close_exited();
        assert!(closed.active_closed);
        assert_eq!(tabs.active_tab().unwrap().title(), "third");
        tabs.tabs[1].exited = true;
        assert!(tabs.close_exited().active_closed);
        assert_eq!(tabs.active_tab().unwrap().title(), "first");
    }

    #[test]
    fn simultaneous_exits_empty_tabs_and_late_notifications_do_not_close_new_tabs() {
        let mut tabs = three_tabs();
        for tab in &mut tabs.tabs {
            tab.exited = true;
        }
        let closed = tabs.close_exited();
        assert!(closed.any_closed);
        assert!(closed.active_closed);
        assert!(tabs.is_empty());
        assert_eq!(tabs.active_index(), 0);
        tabs.push(FakeTab::new());
        let closed = tabs.close_exited();
        assert!(!closed.any_closed);
        assert!(!closed.active_closed);
        assert_eq!(tabs.titles(), ["fake"]);
    }

    #[test]
    fn tab_title_uses_shell_program_name() {
        assert_eq!(tab_title_for_program("nu"), "nu");
        assert_eq!(tab_title_for_program("C:\\tools\\nu.exe"), "nu");
    }
}
