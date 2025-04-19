use crate::vo::Configuration;
mod vo;
// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn start_agent(configuration: Configuration) {
    // ppaass_agent_core::start_server()
    println!("Receive configuration: {:#?}", configuration);
}
#[tauri::command]
fn stop_agent() {}
#[tauri::command]
fn import_users() {}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            start_agent,
            stop_agent,
            import_users
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
