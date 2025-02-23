use nwd::NwgUi;
use nwg::{NativeUi, NwgError};

#[derive(Default)]
pub struct PpaassAgentApplication {
    window: nwg::Window,
    user_combo_box: nwg::ComboBox<&'static str>,
    start_button: nwg::Button,
    stop_button: nwg::Button,
}

impl PpaassAgentApplication {
    fn start(&self) {
        nwg::simple_message(
            "Start agent",
            &format!(
                "Start agent with user: {:?}",
                self.user_combo_box.selection_string()
            ),
        );
    }

    fn stop(&self) {
        nwg::simple_message(
            "Stop agent",
            &format!(
                "Stop agent for user: {:?}",
                self.user_combo_box.selection_string()
            ),
        );
    }

    fn on_close(&self) {
        nwg::simple_message(
            "Close agent",
            &format!(
                "Close agent for user: {:?}",
                self.user_combo_box.selection_string()
            ),
        );
        nwg::stop_thread_dispatch();
    }
}

impl NativeUi<PpaassAgentApplication> for PpaassAgentApplication {
    fn build_ui(mut appication: Self) -> Result<PpaassAgentApplication, NwgError> {
        nwg::Window::builder()
            .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE)
            .position((300, 300))
            .size((800, 600))
            .build(&mut appication.window)?;
        nwg::ComboBox::builder()
            .position((10, 10))
            .parent(&appication.window)
            .build(&mut appication.user_combo_box)?;
        Ok(appication)
    }
}
