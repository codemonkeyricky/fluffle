use crate::error::Result;

pub struct App;

impl App {
    pub async fn new() -> Result<Self> {
        Ok(Self)
    }
}