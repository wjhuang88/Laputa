pub trait Message : Copy + Send {}

#[derive(Copy, Clone)]
pub struct LocalMessage;
impl LocalMessage {
    pub fn new() -> LocalMessage {
        LocalMessage {}
    }
}
impl Message for LocalMessage {}

#[derive(Copy, Clone)]
pub enum MessageSelector {
    Default(LocalMessage)
}
impl Default for MessageSelector {
    fn default() -> Self {
        MessageSelector::Default(LocalMessage::new())
    }
}