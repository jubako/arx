mod app_model;
mod arx_model;
pub use app_model::AppModel;
pub use arx_model::ArxModel;

pub trait Model {
    type Action;
    fn update(&mut self, action: Self::Action);
}
