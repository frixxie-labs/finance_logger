use anyhow::Result;
use yfinance_rs::{DownloadBuilder, YfClient};

use crate::finance::{FinanceApi, Ticker};

pub struct FinanceClient {
    yfin_client: YfClient,
}

impl FinanceClient {
    pub fn new() -> Self {
        Self {
            yfin_client: YfClient::default(),
        }
    }
}

impl FinanceApi for FinanceClient {
    async fn get_tickers(&self, symbols: &[&str]) -> Result<Vec<Ticker>> {
        let symbols = symbols
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        let results = DownloadBuilder::new(&self.yfin_client)
            .symbols(symbols)
            .run()
            .await?;

        let tickers = results
            .entries
            .iter()
            .map(|entry| Ticker::from_candles(entry.instrument.symbol_str(), &entry.history.candles))
            .collect();

        Ok(tickers)
    }
}
