//! Pictures, copied from the Carmen game's `src/art` (see the README) and
//! freed of macroquad: the device paints into a `Canvas` and sends its
//! pixels to the screen itself. `postcard` is the device's own: it fits the
//! game's postcards to the smaller picture frame; so are `computer`, `map`
//! and `printer`, redrawn for that frame.

pub mod computer;
pub mod landmarks;
pub mod map;
pub mod people;
pub mod pixels;
pub mod places;
pub mod postcard;
pub mod printer;
pub mod scene;
pub mod sprites;
