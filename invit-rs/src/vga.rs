use invit::demo::{Demo, SCAN_ESC, SCAN_LEFT, SCAN_RIGHT};
use invit::intro::{HEIGHT, WIDTH};
use minifb::{Key, Scale, ScaleMode, Window, WindowOptions};

/// Mode 13h vises i 4:3, så 320x200 strekkes til 640x480.
const OUT_W: usize = 640;
const OUT_H: usize = 480;

/// Brukeren lukket vinduet.
pub struct Closed;

/// Viser emulert VGA-skjerm i et vindu, låst til 70 Hz.
pub struct Vga {
    window: Window,
    rgba: Vec<u8>,
    out: Vec<u32>,
}

impl Vga {
    pub fn new(title: &str) -> Vga {
        let mut window = Window::new(
            title,
            OUT_W,
            OUT_H,
            WindowOptions {
                resize: true,
                scale: Scale::X2,
                scale_mode: ScaleMode::AspectRatioStretch,
                ..WindowOptions::default()
            },
        )
        .expect("kunne ikke åpne vindu");
        window.set_target_fps(70);
        Vga {
            window,
            rgba: vec![0; WIDTH * HEIGHT * 4],
            out: vec![0; OUT_W * OUT_H],
        }
    }

    /// Venter på neste retrace og viser skjermen.
    pub fn retrace(&mut self, demo: &Demo) -> Result<(), Closed> {
        demo.render_rgba(&mut self.rgba);
        for oy in 0..OUT_H {
            let src = &self.rgba[(oy * HEIGHT / OUT_H) * WIDTH * 4..][..WIDTH * 4];
            let dst = &mut self.out[oy * OUT_W..][..OUT_W];
            for (x, p) in src.chunks_exact(4).enumerate() {
                let c = (p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32;
                dst[x * 2] = c;
                dst[x * 2 + 1] = c;
            }
        }
        self.window
            .update_with_buffer(&self.out, OUT_W, OUT_H)
            .map_err(|_| Closed)?;
        if self.window.is_open() { Ok(()) } else { Err(Closed) }
    }

    /// Det introen leste fra tastaturkontrolleren på port 60h.
    pub fn scancode(&self) -> u8 {
        if self.window.is_key_down(Key::Escape) {
            SCAN_ESC
        } else if self.window.is_key_down(Key::Left) {
            SCAN_LEFT
        } else if self.window.is_key_down(Key::Right) {
            SCAN_RIGHT
        } else {
            0
        }
    }
}
