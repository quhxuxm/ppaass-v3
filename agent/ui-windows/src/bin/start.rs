use nwg::NativeUi;
use ppaass_agent_ui_windows::app::BasicApp;
fn main() {
    nwg::init().expect("Failed to init Native Windows GUI");
    let _app = BasicApp::build_ui(Default::default()).expect("Failed to build UI");
    nwg::dispatch_thread_events();
}
