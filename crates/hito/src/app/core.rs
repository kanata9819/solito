use std::{collections::HashMap, error::Error, sync::Arc};

use tracing::error;
use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::PhysicalKey,
    window::{Window, WindowAttributes, WindowId},
};

use crate::renderer::state::State;

pub struct HitoApplication {
    pub windows: HashMap<WindowId, Arc<Window>>,
    state: Option<State>,
}

impl HitoApplication {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            state: None,
        }
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        let window_attributes: WindowAttributes = WindowAttributes::default().with_title("Hito");

        let window: Arc<Window> = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("failed to create window"),
        );

        let window_id: WindowId = window.id();
        self.windows.insert(window_id, window.clone());

        let mut state: State = pollster::block_on(State::new(window))?;
        state.render()?;
        self.state = Some(state);

        Ok(())
    }
}

impl ApplicationHandler for HitoApplication {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(err) = self.create_window(event_loop) {
            error!("create window error occured: {}", err)
        };

        if let Some(state) = &mut self.state {
            state.resize(1200, 750);
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: ()) {}

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let state: &mut State = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => {
                let _ = self.windows.remove(&window_id);
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                state.update();
                if let Err(e) = state.render() {
                    error!("{e}");
                    event_loop.exit();
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => state.handle_key(event_loop, code, key_state.is_pressed()),
            _ => {}
        }
    }
}

// impl ApplicationHandler<State> for HitoApplication {
//     fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
//         // remove `todo!()`
//     }

//     fn window_event(
//         &mut self,
//         event_loop: &ActiveEventLoop,
//         _window_id: winit::window::WindowId,
//         event: WindowEvent,
//     ) {
//         let state: &mut State = match &mut self.state {
//             Some(canvas) => canvas,
//             None => return,
//         };

//         match event {
//             // ...
//             WindowEvent::KeyboardInput {
//                 event:
//                     KeyEvent {
//                         physical_key: PhysicalKey::Code(code),
//                         state: key_state,
//                         ..
//                     },
//                 ..
//             } => state.handle_key(event_loop, code, key_state.is_pressed()),

//             WindowEvent::RedrawRequested => {
//                 state.update();
//                 match state.render() {
//                     Ok(_) => {}
//                     Err(e) => {
//                         // Log the error and exit gracefully
//                         error!("{e}");
//                         event_loop.exit();
//                     }
//                 }
//             }
//             _ => {}
//         }
//     }
// }
