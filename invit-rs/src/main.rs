//! Native versjon: vindu via minifb og musikk via xmrsplayer/cpal.

mod music;
mod vga;

use invit::demo::{CREDIT_TEXT, Demo, EXIT_TEXT, MUSIC};
use vga::Vga;

fn main() {
    let mut demo = Demo::new();
    let mut vga = Vga::new("ROMANTICA'98 invitro - The Phrenetics");
    let music = match music::play(MUSIC) {
        Ok(m) => Some(m),
        Err(e) => {
            eprintln!("{e} - fortsetter uten musikk");
            None
        }
    };

    let mut completed = false;
    while vga.retrace(&demo).is_ok() {
        demo.tick(vga.scancode());
        if demo.finished() {
            completed = true;
            break;
        }
    }

    drop(vga);
    if completed {
        println!("{EXIT_TEXT}");
        println!();
    }
    drop(music);
    println!("{CREDIT_TEXT}");
    println!();
}
