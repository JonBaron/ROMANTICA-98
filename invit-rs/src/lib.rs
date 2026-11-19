//! ROMANTICA'98 invitasjonsintro av The Phrenetics (1998).
//! Portert fra TASM32/DOS32 til Rust. Effektene, tallene og rekkefølgen følger
//! originalen; VGA-kortet, tastaturporten og lydkortet emuleres av verten
//! (vindu + cpal i den native versjonen, canvas + Web Audio i nettleseren).

pub mod demo;
pub mod intro;
pub mod pcx;

#[cfg(target_arch = "wasm32")]
mod web;
