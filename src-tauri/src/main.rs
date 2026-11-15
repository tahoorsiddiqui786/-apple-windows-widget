// Release builds run as a GUI app (no console window). Debug builds keep the
// console so `npm run app:dev` shows native log output.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    glass_desktop_lib::run();
}
