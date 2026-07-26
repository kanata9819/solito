//! Execute application commands after input has been translated.

use super::{AppResult, SolitoApplication};
use crate::{
    app::input::{AppCommand, CopyModeCommand},
    session::runtime::SessionInput,
};
use solito_renderer::TerminalSize;
use winit::event_loop::ActiveEventLoop;

impl SolitoApplication {
    fn activate_next_tab(&mut self) {
        if self.tabs.activate_next() {
            self.refresh_tab_bar();
            self.show_active_terminal_at_bottom();
        }
    }

    fn activate_previous_tab(&mut self) {
        if self.tabs.activate_previous() {
            self.refresh_tab_bar();
            self.show_active_terminal_at_bottom();
        }
    }

    pub(super) fn handle_command(
        &mut self,
        command: AppCommand,
        event_loop: &ActiveEventLoop,
    ) -> AppResult {
        match command {
            AppCommand::Noop => {}
            AppCommand::EnterCopyMode => {
                if let Some(snapshot) = self.tabs.active_snapshot() {
                    self.copy_mode.enter(&snapshot);
                    self.refresh_copy_mode(&snapshot);
                    self.update_window_title();
                }
            }
            AppCommand::CopyMode(command) => self.handle_copy_mode_command(command)?,
            AppCommand::NewTab => {
                self.leave_copy_mode();
                if let Some(renderer) = &mut self.renderer {
                    let size: TerminalSize = renderer.terminal_size();
                    self.tabs.open(size, self.config.shell.program.clone());
                    self.refresh_tab_bar();
                    self.show_active_terminal_at_bottom();
                }
            }
            AppCommand::CloseTab => {
                self.leave_copy_mode();
                if self.tabs.close_active() {
                    if self.tabs.is_empty() {
                        event_loop.exit();
                    } else {
                        self.refresh_tab_bar();
                        self.show_active_terminal_at_bottom();
                    }
                }
            }
            AppCommand::NextTab => {
                self.leave_copy_mode();
                self.activate_next_tab();
            }
            AppCommand::PreviousTab => {
                self.leave_copy_mode();
                self.activate_previous_tab();
            }
            AppCommand::CopySelection => {
                if let Some(snapshot) = self.tabs.active_snapshot()
                    && let Some(text) = self.copy_mode.selected_text(&snapshot)
                {
                    Self::copy_to_clipboard(text)?;
                }
            }
            AppCommand::PasteFromClipboard => self.paste_from_clipboard()?,
            AppCommand::Resize {
                window_size,
                terminal_size,
            } => {
                self.tabs.resize_all(terminal_size)?;

                if let (Some(renderer), Some(snapshot)) =
                    (&mut self.renderer, self.tabs.active_snapshot())
                {
                    let copy_mode = self.copy_mode.renderer_snapshot(&snapshot);
                    renderer.resize(window_size, snapshot);
                    renderer.set_copy_mode(copy_mode);
                }
            }
        }

        Ok(())
    }

    fn handle_copy_mode_command(&mut self, command: CopyModeCommand) -> AppResult {
        let Some(snapshot) = self.tabs.active_snapshot() else {
            return Ok(());
        };

        match command {
            CopyModeCommand::Move(direction) => {
                self.copy_mode.move_cursor(&snapshot, direction);
                self.refresh_copy_mode(&snapshot);
            }
            CopyModeCommand::ToggleCellSelection => {
                self.copy_mode.toggle_cell_selection();
                self.refresh_copy_mode(&snapshot);
            }
            CopyModeCommand::ToggleLineSelection => {
                self.copy_mode.toggle_line_selection();
                self.refresh_copy_mode(&snapshot);
            }
            CopyModeCommand::CopyAndExit => {
                let copy_result = self
                    .copy_mode
                    .selected_text(&snapshot)
                    .map_or(Ok(()), Self::copy_to_clipboard);
                self.leave_copy_mode();
                copy_result?;
            }
            CopyModeCommand::Exit => self.leave_copy_mode(),
        }

        Ok(())
    }

    fn copy_to_clipboard(text: String) -> AppResult {
        let mut clipboard = arboard::Clipboard::new()?;
        clipboard.set_text(text)?;
        Ok(())
    }

    fn paste_from_clipboard(&self) -> AppResult {
        let Some(input_tx) = self.tabs.active_input_tx() else {
            return Ok(());
        };
        let Ok(mut clipboard) = arboard::Clipboard::new() else {
            return Ok(());
        };
        let Ok(text) = clipboard.get_text() else {
            return Ok(());
        };

        if !text.is_empty() {
            input_tx.send(SessionInput::write(text.into_bytes()))?;
        }

        Ok(())
    }
}
