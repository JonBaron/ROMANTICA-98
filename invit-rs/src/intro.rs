use crate::pcx::Pcx;

pub const WIDTH: usize = 320;
pub const HEIGHT: usize = 200;

const SIN_AMP: i16 = 0xDFFFu16 as i16;
const PAGES_STRIDE: usize = 5 * WIDTH;

pub const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz{}1234567890:.'/!-@";
pub const SCROLL_TEXT: &[u8] = b"      {[invitation]}\\_you are hereby wellcome to:\\   romantica'98 !!\\_ date : 20-22 feb.\\place : porsgrunn\\price : 200.-\\_arranged by tp n' dnf\\>        {[where]}\\_the party is located in\\porsgrunn in norway.\\vestsiden ungdumskole.\\see roadmap on webpage\\or mail the organizers\\for further details.\\centrum 250m away.\\>       {[compos]}\\_appart from the normal\\democompos there will\\also be lotsa fun and\\fast compos. like\\swappercompo. ateam.\\coladrinking. joik.\\live codecompo. etc!\\>       {[stuff]}\\_visible alcohole will be\\confiscated. toxicated\\people will be sent\\on a joggetur.\\rasism and personal\\injuries will not be\\tolerated\\>       {[contact]}\\_web page:\\www.neutralzone.org -\\/romantica\\_mail:\\ hermanse@online.no\\_see you there!\\";

/// x-grensene for hver bokstav i ROMANTICA'98.
const LOGO_WIDTHS: [u16; 12] = [0, 22, 49, 80, 108, 141, 168, 176, 193, 220, 239, 276];

pub struct Intro {
    pub font_gfx: Vec<u8>,
    pub font_width: usize,
    pub flower_gfx: Vec<u8>,
    pub vscreen: Vec<u8>,
    pub earth_tex: Vec<u8>,
    pub banner_gfx: Vec<u8>,
    pub tunnel_tab: Vec<u8>,
    pub text_pages: Vec<u8>,
    pub logo_mask: Vec<u8>,
    pub intro_tex: Vec<u8>,
    pub intro_pal: [u8; 768],
    pub main_pal: [u8; 768],

    sin: [i16; 513],
    cos: [i16; 513],
    shade: [u8; 256],

    pub fade: i32,
    pub frame: u32,
    pub esc_timer: i32,
    pub quit_count: i32,
    pub left_timer: u8,
    pub right_timer: u8,
    pub flower_frame: usize,
    pub scroll_x: i32,
    pub scroll_target: i32,
    pub logo_width: usize,
}

impl Intro {
    pub fn new() -> Intro {
        Intro {
            font_gfx: vec![0; 0x5208],
            font_width: 0,
            flower_gfx: vec![0; 0x9E34],
            vscreen: vec![0; WIDTH * HEIGHT],
            earth_tex: vec![0; 90000],
            banner_gfx: vec![0; 0x11800],
            tunnel_tab: vec![0; 0x20000],
            text_pages: vec![0; 5 * WIDTH * HEIGHT],
            logo_mask: vec![0; 0x2580],
            intro_tex: vec![0; 256 * 256],
            intro_pal: [0; 768],
            main_pal: [0; 768],
            sin: [0; 513],
            cos: [0; 513],
            shade: [0; 256],
            fade: 1,
            frame: 0,
            esc_timer: 0,
            quit_count: 0,
            left_timer: 0,
            right_timer: 0,
            flower_frame: 4,
            scroll_x: 0,
            scroll_target: 0,
            logo_width: 1,
        }
    }

    /// Den originale rutinen kopierer bare 255 farger, og nullstiller alltid
    /// farge 0-2 i hovedpaletten, også når målet er intropaletten.
    pub fn copy_palette(&mut self, pcx: &Pcx, to_intro: bool) {
        let dst = if to_intro { &mut self.intro_pal } else { &mut self.main_pal };
        for i in 0..765 {
            dst[i] = pcx.palette[i] >> 2;
        }
        self.main_pal[..9].fill(0);
    }

    /// Skriver paletten, skalert med `fade`, til de 256 DAC-registrene (6 bit per farge).
    pub fn set_faded_palette(&self, dac: &mut [u8; 768], intro: bool) {
        let pal = if intro { &self.intro_pal } else { &self.main_pal };
        let fade = self.fade as u32;
        for (d, &v) in dac.iter_mut().zip(pal.iter()) {
            *d = ((v as u32).wrapping_mul(fade) >> 8).min(63) as u8;
        }
    }

    pub fn build_sin_tables(&mut self) {
        let amp = SIN_AMP as f64;
        for i in 0..=0x200 {
            self.sin[i] = (((i * 4) as f64 / 326.0).sin() * amp).round_ties_even() as i16;
            self.cos[i] = ((((i + 128) * 4) as f64 / 326.0).sin() * amp).round_ties_even() as i16;
        }
    }

    pub fn build_tunnel_table(&mut self) {
        let mut t = 0;
        for row in (1..=200i32).rev() {
            for col in (1..=256i32).rev() {
                let d = (col + 32) as i16;
                let u = ((-(row - 100) * 128) as i16 / d) as u8;
                let v = (0x2400i16 / d) as u8;
                self.tunnel_tab[t] = u.wrapping_add(0x7F);
                self.tunnel_tab[t + 1] = v.wrapping_add(0xE0);
                t += 2;
            }
        }
    }

    pub fn build_shade_table(&mut self) {
        self.shade = [0; 256];
        for k in 0..192 {
            self.shade[64 + k] = (k / 16) as u8;
        }
    }

    /// Tegner hele invitasjonsteksten inn i fem sider ved siden av hverandre (1600x200).
    pub fn render_text(&mut self) {
        let mut page = 0usize;
        let mut x = 8usize;
        let mut y = 35usize;
        let mut bold = 0usize;
        for &ch in SCROLL_TEXT {
            let mut c = ch;
            match c {
                b'>' => {
                    page += WIDTH;
                    x = 8;
                    y = 35;
                    continue;
                }
                b'[' => {
                    bold = 3;
                    continue;
                }
                b']' => {
                    bold = 0;
                    continue;
                }
                b'_' => {
                    y += 8;
                    x = 0;
                    c = 0;
                }
                b'\\' => {
                    y += 17;
                    x = 0;
                    c = 0;
                }
                _ => {}
            }
            if c > b' ' {
                if let Some(idx) = CHARSET.iter().position(|&k| k == c) {
                    let stride = self.font_width + 1;
                    let glyph = self.font_width + idx * 14 + 2;
                    let dst = page + y * PAGES_STRIDE + x;
                    for gy in 0..16 + bold {
                        for gx in 0..13 {
                            let p = self.font_gfx[glyph + gy * stride + gx];
                            if p != 0 {
                                self.text_pages[dst + gy * PAGES_STRIDE + gx] = p;
                            }
                        }
                    }
                }
            }
            x += 11;
        }
    }

    /// "Vinglende" rotozoomer over st_bak.pcx.
    pub fn roto_zoom(&mut self) {
        let s = self.sin[(self.frame & 511) as usize] as i32;
        let c = self.cos[((self.frame as i32).wrapping_add(s >> 4) & 511) as usize] as i32;
        let (s8, c8) = (s << 3, c << 3);
        let (mut u_row, mut v_row) = (c << 5, s << 5);
        for y in 0..HEIGHT {
            let (mut u, mut v) = (u_row, v_row);
            for x in 0..WIDTH {
                let ah = ((u as u32) >> 16) as u8;
                let e = (v as u32) >> 16;
                let tx = e as u8;
                let ty = ((e >> 8) as u8).wrapping_add(ah);
                self.vscreen[y * WIDTH + x] = self.intro_tex[(ty as usize) << 8 | tx as usize];
                u = u.wrapping_add(s8);
                v = v.wrapping_sub(c8);
            }
            u_row = u_row.wrapping_add(c8);
            v_row = v_row.wrapping_add(s8);
        }
    }

    /// Stanser ut ROMANTICA'98 i svart, bokstav for bokstav.
    pub fn draw_logo_mask(&mut self) {
        let dst = 84 * WIDTH + 22;
        for y in 0..32 {
            for x in 0..self.logo_width {
                if self.logo_mask[y * 276 + x] == 1 {
                    self.vscreen[dst + y * WIDTH + x] = 0;
                }
            }
        }
    }

    pub fn draw_tunnel(&mut self) {
        let f = self.frame as u8;
        for y in 0..HEIGHT {
            for x in 0..128 {
                let t = y * 512 + x * 4;
                let u = self.tunnel_tab[t];
                let v = self.tunnel_tab[t + 1].wrapping_add(f);
                let c = self.earth_tex[(v as usize) << 8 | u as usize];
                let lo = (c & 15).saturating_sub(self.shade[x * 2]);
                let p = (c & 0xF0) + lo;
                self.vscreen[y * WIDTH + x * 2] = p;
                self.vscreen[y * WIDTH + x * 2 + 1] = p;
            }
        }
    }

    pub fn draw_banner(&mut self) {
        for i in WIDTH..WIDTH + 38 * WIDTH {
            let p = self.banner_gfx[i];
            if p != 0 {
                self.vscreen[i] = p;
            }
        }
    }

    /// Farge 1 er gjennomsiktig, unntatt i de 64 kolonnene lengst til høyre.
    pub fn draw_text_page(&mut self) {
        let src = 35 * PAGES_STRIDE + (self.scroll_x * 2) as usize;
        let dst = 45 * WIDTH;
        for y in 0..144 {
            for x in 0..WIDTH {
                let p = self.text_pages[src + y * PAGES_STRIDE + x];
                if x >= 256 || p != 1 {
                    self.vscreen[dst + y * WIDTH + x] = p;
                }
            }
        }
    }

    pub fn draw_flower(&mut self) {
        let src = self.flower_frame * 90;
        let dst = 115 * WIDTH + 232;
        for y in 0..85 {
            for x in 0..88 {
                let p = self.flower_gfx[src + y * 450 + x];
                if p != 0 {
                    self.vscreen[dst + y * WIDTH + x] = p;
                }
            }
        }
    }

    pub fn update_logo_reveal(&mut self) {
        if self.fade >= 0xF0 && self.frame >= 0x80 {
            let i = ((self.frame - 0x7F) >> 4) as usize + 1;
            if i >= 17 {
                self.esc_timer += 1;
            }
            if i <= 11 {
                self.logo_width = LOGO_WIDTHS[i] as usize;
            }
        }
    }
}
