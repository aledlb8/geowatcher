//! `GeoWatcher`: watches Geometry Dash streams and alerts on real runs.

pub mod app;
pub mod appearance;
pub mod capture;
pub mod config;
pub mod detect;
pub mod player;
pub mod run;
pub mod supervisor;
pub mod twitch;
pub mod widget;

pub use app::run;
