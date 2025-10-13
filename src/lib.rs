use crate::presence::Presences;

pub mod presence;
pub mod api;

pub struct AppState {
    pub presences: Presences
}