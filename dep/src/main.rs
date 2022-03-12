#![windows_subsystem = "windows"]
use iced::{Application, Settings};
mod error;
mod logic;
mod ui;
use crate::ui::App;
use error::ApplicationError;
fn main() -> Result<(), ApplicationError> {
    App::run(Settings {
        default_font: Some(include_bytes!("../fonts/simhei.ttf")),
        ..Default::default()
    })?;
    Ok(())
}
