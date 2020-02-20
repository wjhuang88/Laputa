pub trait LifeCycleHandler : Copy + Send {}

#[derive(Copy, Clone)]
pub struct DefaultLifeCycleHandler;
impl DefaultLifeCycleHandler {
    pub fn new() -> DefaultLifeCycleHandler {
        DefaultLifeCycleHandler {}
    }
}
impl LifeCycleHandler for DefaultLifeCycleHandler {}

#[derive(Copy, Clone)]
pub enum LifeCycleSelector {
    Default(DefaultLifeCycleHandler)
}
impl Default for LifeCycleSelector {
    fn default() -> Self {
        LifeCycleSelector::Default(DefaultLifeCycleHandler::new())
    }
}
