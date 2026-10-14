// KeyBox is a GUI application; double-clicking either build should not open a console.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() {
    keybox_lib::run()
}
