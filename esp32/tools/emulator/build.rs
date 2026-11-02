//! Makes what the game includes from `$OUT_DIR` with the device's own
//! `build.rs` (fonts, sprites, photos, coastlines), run from the repository's
//! root, and takes the sound effects' notes out of `src/device/sound.rs`,
//! whose player needs the device.

#[allow(dead_code)]
#[path = "../../build.rs"]
mod device;

fn main() {
    std::env::set_current_dir("../..").expect("the repository's root");
    device::assets();
    let sound = std::fs::read_to_string("src/device/sound.rs").expect("the sound effects");
    let from = sound.find("/// (pitch in Hz").expect("the notes' type");
    let to = sound.find("/// Whether sound is on").expect("the end of the effects");
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    std::fs::write(out.join("effects.rs"), &sound[from..to]).unwrap();
}
