#[derive(Clone, Copy, Debug)]
pub(crate) enum AppEvent {
    TerminalOutputReady,
}

/// Coalesce many PTY reads into one pending UI wake-up per tab.
#[derive(Clone)]
pub(crate) struct OutputNotifier {
    proxy: winit::event_loop::EventLoopProxy<AppEvent>,
    pending: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl OutputNotifier {
    pub(crate) fn new(proxy: winit::event_loop::EventLoopProxy<AppEvent>) -> Self {
        Self {
            proxy,
            pending: Default::default(),
        }
    }

    pub(crate) fn notify(&self) -> bool {
        if self.pending.swap(true, std::sync::atomic::Ordering::AcqRel) {
            return true;
        }
        self.proxy.send_event(AppEvent::TerminalOutputReady).is_ok()
    }

    pub(crate) fn begin_drain(&self) {
        // Clear before receiving: a racing producer must be able to wake us again.
        self.pending
            .store(false, std::sync::atomic::Ordering::Release);
    }
}
