use accounts::actors::{
    account::{ChargebackRequest, DepositRequest, DisputeRequest, ResolveRequest, WithdrawRequest},
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

async fn process_line(shard: AccountShardClient, record: CsvRecord) {
    let CsvRecord {
        t,
        client,
        tx,
        amount,
    } = record;

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
        "resolve" => {
            let _ = shard
                .send_account_async(ResolveRequest {
                    account_id: client,
                    transaction_id: tx,
                })
                .await;
        }
        "chargeback" => {
            let _ = shard
                .send_account_async(ChargebackRequest {
                    account_id: client,
                    transaction_id: tx,
                })
                .await;
        }
        t => tracing::warn!("Unkown command: {}", t),
    };
}

pub async fn process(shard: AccountShardClient, input: String) {
    let mut reader = ReaderBuilder::new()
        .delimiter(b',')
        .has_headers(true)
        .trim(Trim::All)
        .from_path(input)
        .unwrap(); //TODO unwrap

    let mut tasks = vec![];
    for result in reader.deserialize() {
        let record: CsvRecord = result.unwrap();
        let shard = shard.clone();

        let t = tokio::task::spawn(async move {
            let _ = process_line(shard, record).await;
        });
        tasks.push(t);
    }

    for t in tasks {
        let _ = t.await;
    }
}
