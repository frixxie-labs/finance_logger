use anyhow::Result;

mod finance_client;
mod types;

pub use finance_client::FinanceClient;
pub use types::{Ticker, finance_device};

pub trait FinanceApi {
    async fn get_tickers(&self, symbol: &[&str]) -> Result<Vec<Ticker>>;
}
