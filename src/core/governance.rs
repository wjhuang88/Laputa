pub trait Governance : Copy + Send {}

#[derive(Copy, Clone)]
pub struct DefaultGovernance;
impl DefaultGovernance {
    pub fn new() -> DefaultGovernance {
        DefaultGovernance {}
    }
}
impl Governance for DefaultGovernance {}

#[derive(Copy, Clone)]
pub enum GovernanceSelector {
    Default(DefaultGovernance)
}
impl Default for GovernanceSelector {
    fn default() -> Self {
        GovernanceSelector::Default(DefaultGovernance::new())
    }
}