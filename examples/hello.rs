use laputa::common;
use async_std::task;

fn main() -> common::Result<()> {
    task::block_on(async {
        laputa::new().start().await?;
        Ok(())
    })
}