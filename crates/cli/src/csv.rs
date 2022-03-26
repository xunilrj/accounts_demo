use accounts::actors::{
    account::{DepositRequest, DisputeRequest, WithdrawRequest},
    account_shard::AccountShardClient,
};
use csv::{ReaderBuilder, Trim};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CsvRecord {
    #[serde(rename = "type")]
    t: String,
    client: u32,
    tx: u32,
    amount: Option<f64>,
}

pub async fn process(shard: AccountShardClient, input: String) {
    let mut reader = ReaderBuilder::new()
        .delimiter(b',')
        .has_headers(true)
        .trim(Trim::All)
        .from_path(input)
        .unwrap(); //TODO unwrap
    for result in reader.deserialize() {
        let CsvRecord {
            t,
            client,
            amount,
            tx,
        } = result.unwrap();

        let t = t.to_ascii_lowercase();

        use accounts::domain::money::Currency::*;
        match t.as_str() {
            "deposit" => {
                let _ = shard
                    .send_account_async(DepositRequest {
                        account_id: client,
                        transaction_id: tx,
                        amount: amount.unwrap() * Bitcoin,
                    })
                    .await;
            }
            "withdrawal" => {
                let _ = shard
                    .send_account_async(WithdrawRequest {
                        account_id: client,
                        transaction_id: tx,
                        amount: amount.unwrap() * Bitcoin,
                    })
                    .await;
            }
            "dispute" => {
                let _ = shard
                    .send_account_async(DisputeRequest {
                        account_id: client,
                        transaction_id: tx,
                    })
                    .await;
            }
            t => tracing::warn!("Unkown command: {}", t),
        };
    }

    //TODO we need a guarantee that all messages were processed.
}
