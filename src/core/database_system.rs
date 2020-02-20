pub trait DatabaseSystem : Copy + Send {}

#[derive(Copy, Clone)]
pub struct MockDatabaseSystem;
impl MockDatabaseSystem {
    pub fn new() -> MockDatabaseSystem {
        MockDatabaseSystem {}
    }
}
impl DatabaseSystem for MockDatabaseSystem {}

#[derive(Copy, Clone)]
pub enum DatabaseSelector {
    Default(MockDatabaseSystem)
}
impl Default for DatabaseSelector {
    fn default() -> Self {
        DatabaseSelector::Default(MockDatabaseSystem::new())
    }
}