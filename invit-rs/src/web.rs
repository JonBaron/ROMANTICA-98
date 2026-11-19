//! Rå WebAssembly-grensesnitt uten wasm-bindgen; se `web/main.js`.

use crate::demo::Demo;
use crate::intro::{HEIGHT, WIDTH};
use std::cell::RefCell;

struct State {
    demo: Demo,
    rgba: Vec<u8>,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

#[unsafe(no_mangle)]
pub extern "C" fn demo_init() {
    STATE.with_borrow_mut(|s| {
        *s = Some(State {
            demo: Demo::new(),
            rgba: vec![0; WIDTH * HEIGHT * 4],
        })
    });
}

/// Kjører én retrace. Returnerer 0 når introen er ferdig.
#[unsafe(no_mangle)]
pub extern "C" fn demo_tick(scancode: u32) -> u32 {
    STATE.with_borrow_mut(|s| {
        let s = s.as_mut().expect("demo_init er ikke kalt");
        s.demo.tick(scancode as u8);
        !s.demo.finished() as u32
    })
}

/// Peker til et 320x200 RGBA-bilde av skjermen.
#[unsafe(no_mangle)]
pub extern "C" fn demo_frame() -> *const u8 {
    STATE.with_borrow_mut(|s| {
        let s = s.as_mut().expect("demo_init er ikke kalt");
        let State { demo, rgba } = s;
        demo.render_rgba(rgba);
        rgba.as_ptr()
    })
}
