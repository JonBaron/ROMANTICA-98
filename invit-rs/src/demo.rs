//! Introens hovedløkker som en tilstandsmaskin: hvert kall til `tick` tilsvarer
//! én vertikal retrace (70 Hz) i originalen.

use crate::intro::{HEIGHT, Intro, WIDTH};
use crate::pcx;

pub const MUSIC: &[u8] = include_bytes!("../../DATA/HIGH_HPS.XM");

const FONT26: &[u8] = include_bytes!("../../DATA/FONT26.PCX");
const BANNER2: &[u8] = include_bytes!("../../DATA/BANNER2.PCX");
const BLOMSTER: &[u8] = include_bytes!("../../DATA/BLOMSTER.PCX");
const EARTH2: &[u8] = include_bytes!("../../DATA/EARTH2.PCX");
const ST_BAK: &[u8] = include_bytes!("../../DATA/ST_BAK.PCX");
const ROMINTRO: &[u8] = include_bytes!("../../DATA/ROMINTRO.PCX");

pub const SCAN_ESC: u8 = 0x01;
pub const SCAN_LEFT: u8 = 0x4B;
pub const SCAN_RIGHT: u8 = 0x4D;

pub const EXIT_TEXT: &str = "det var egentlig alt det... bare du scrolla!";
pub const CREDIT_TEXT: &str = " a TP-multimedia produksjon tm";

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    /// Musikken har startet, skjermen er svart i 20 retracer.
    Startup(u32),
    Part1,
    /// Svart pause i 60 retracer mellom del 1 og del 2.
    Pause(u32),
    Part2,
    Done,
}

pub struct Demo {
    fx: Intro,
    phase: Phase,
    /// Det som ligger i videominnet på 0xA0000.
    pub vram: Vec<u8>,
    /// DAC-registrene, 6 bit per farge.
    pub dac: [u8; 768],
}

impl Demo {
    pub fn new() -> Demo {
        let mut fx = Intro::new();
        let font = pcx::decode(FONT26, &mut fx.font_gfx);
        fx.font_width = font.width;
        pcx::decode(BANNER2, &mut fx.banner_gfx);
        pcx::decode(BLOMSTER, &mut fx.flower_gfx);
        let earth = pcx::decode(EARTH2, &mut fx.earth_tex);
        fx.copy_palette(&earth, false);
        let st_bak = pcx::decode(ST_BAK, &mut fx.intro_tex);
        fx.copy_palette(&st_bak, true);
        pcx::decode(ROMINTRO, &mut fx.logo_mask);

        fx.vscreen.fill(1);
        fx.text_pages.fill(1);
        fx.build_tunnel_table();
        fx.build_shade_table();
        fx.render_text();
        fx.build_sin_tables();

        Demo {
            fx,
            phase: Phase::Startup(20),
            vram: vec![0; WIDTH * HEIGHT],
            dac: [0; 768],
        }
    }

    pub fn finished(&self) -> bool {
        self.phase == Phase::Done
    }

    /// Kjører fram til neste retrace. `scancode` er det som ligger på port 60h.
    pub fn tick(&mut self, scancode: u8) {
        let fx = &mut self.fx;
        match self.phase {
            Phase::Startup(n) => {
                self.phase = if n > 1 { Phase::Startup(n - 1) } else { Phase::Part1 };
                if self.phase == Phase::Part1 {
                    fx.fade = 5;
                    fx.frame = 0;
                }
            }
            Phase::Part1 => {
                fx.roto_zoom();
                fx.draw_logo_mask();
                fx.set_faded_palette(&mut self.dac, true);
                self.vram.copy_from_slice(&fx.vscreen);
                fx.frame += 1;

                fx.update_logo_reveal();
                if scancode == SCAN_ESC {
                    fx.esc_timer += 1;
                }
                if fx.esc_timer >= 1 {
                    fx.fade -= 2;
                } else if fx.fade < 0xFE {
                    fx.fade += 2;
                }
                if fx.fade < 5 {
                    fx.esc_timer = 0;
                    fx.fade = 0;
                    fx.set_faded_palette(&mut self.dac, true);
                    fx.vscreen.fill(0);
                    self.phase = Phase::Pause(60);
                }
            }
            Phase::Pause(n) => {
                if n > 1 {
                    self.phase = Phase::Pause(n - 1);
                } else {
                    fx.fade = 2;
                    fx.scroll_target = 0;
                    fx.frame = 0;
                    self.phase = Phase::Part2;
                }
            }
            Phase::Part2 => {
                fx.draw_tunnel();
                fx.draw_banner();
                fx.draw_text_page();
                fx.draw_flower();
                if fx.quit_count == 0 && fx.fade < 0xFF {
                    fx.fade += 1;
                }
                fx.set_faded_palette(&mut self.dac, false);
                self.vram.copy_from_slice(&fx.vscreen);
                if part2_logic(fx, scancode) {
                    self.phase = Phase::Done;
                }
            }
            Phase::Done => {}
        }
    }

    /// Konverterer videominnet til RGBA via DAC-paletten.
    pub fn render_rgba(&self, out: &mut [u8]) {
        let mut rgb = [[0u8; 4]; 256];
        for (i, c) in rgb.iter_mut().enumerate() {
            let to8 = |v: u8| (v << 2) | (v >> 4);
            *c = [to8(self.dac[i * 3]), to8(self.dac[i * 3 + 1]), to8(self.dac[i * 3 + 2]), 255];
        }
        for (px, &p) in out.chunks_exact_mut(4).zip(self.vram.iter()) {
            px.copy_from_slice(&rgb[p as usize]);
        }
    }
}

/// Alt i del 2 som skjer etter retrace. Returnerer `true` når introen er ferdig.
fn part2_logic(fx: &mut Intro, scancode: u8) -> bool {
    if fx.esc_timer != 0 {
        fx.esc_timer -= 1;
        if fx.esc_timer == 0 {
            fx.flower_frame = 4;
        }
    }
    if fx.left_timer != 0 {
        fx.left_timer -= 1;
        if fx.left_timer == 0 {
            fx.flower_frame = 4;
        }
    }
    if fx.right_timer != 0 {
        fx.right_timer -= 1;
        if fx.right_timer == 0 {
            fx.flower_frame = 4;
        }
    }

    if fx.scroll_x < fx.scroll_target {
        fx.scroll_x += 8;
    } else if fx.scroll_x > fx.scroll_target {
        fx.scroll_x -= 8;
    }
    fx.frame += 1;

    match scancode {
        SCAN_ESC => {
            fx.flower_frame = 0;
            fx.esc_timer = 25;
            fx.quit_count += 1;
        }
        SCAN_LEFT if fx.left_timer == 0 => {
            fx.flower_frame = 1;
            fx.left_timer = 25;
            fx.scroll_target -= 160;
            if fx.scroll_target < 0 {
                fx.scroll_target = 4 * 160;
            }
        }
        SCAN_RIGHT if fx.right_timer == 0 => {
            fx.flower_frame = 2;
            fx.right_timer = 25;
            fx.scroll_target += 160;
            if fx.scroll_target > 4 * 160 {
                fx.scroll_target = 0;
            }
        }
        _ => {}
    }
    if fx.quit_count > 1 {
        fx.fade -= 1;
    }
    if fx.quit_count > 2 {
        fx.fade -= 1;
    }
    fx.fade < 2
}
