//! probeg ("mileage" in Russian) prints lifetime statistics for a Mac.
//!
//! `collect` takes readings, `render` draws the page.
//! The model (`model`) is the only thing connecting them.

pub mod collect;
pub mod fmt;
pub mod i18n;
pub mod model;
pub mod render;
pub mod sample;
pub mod sys;
