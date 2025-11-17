use crate::presence::Presences;

pub mod api;
pub mod presence;

pub struct AppState {
    pub presences: Presences,
}
