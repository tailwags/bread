//! Everything on screen. The modules here are building blocks; `screens`
//! has what `main` and the panic handler show.
//!
//! The panic handler draws with `theme`, `font`, `input`, `console`,
//! `surface`, `chrome` and `report`, so those must stay allocation-free and
//! must not call `effie::system_table()`, which may be what broke.

pub mod canvas;
pub mod chrome;
pub mod console;
pub mod font;
pub mod input;
pub mod report;
pub mod screens;
pub mod surface;
pub mod theme;
