mod csv;

use accounts::actors::aggregators::accounts_state_aggregator::{
    AccountState, AccountsStateActor, AccountsStateAggregator,
};
use accounts::actors::aggregators::AggregatorRequests;
use accounts::actors::Actor;
use accounts::actors::{account_manager::AccountManagerActor, account_shard::AccountShardActor};
use accounts::broadcast::Broadcast;
use argh::FromArgs;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(FromArgs, PartialEq, Debug)]
/// Aggregate accounts final positions
struct Args {
    /// file that will be processed (eg: somefolder/somefile.csv)
    #[argh(positional)]
    input: String,

    /// log verbosity
    #[argh(switch, short = 'v')]
    verbose: bool,
}

fn print_accounts_state(state: &AccountsStateAggregator) {
    println!("client,available,held,total,locked");
    for AccountState {
        client,
        available,
        held,
        total,
        locked,
    } in state.accounts.values()
    {
        println!("{client},{available},{held},{total},{locked}");
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::Registry::default()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(
            tracing_tree::HierarchicalLayer::new(2)
                .with_targets(true)
                .with_bracketed_fields(true),
        )
        .init();

    let args: Args = argh::from_env();

    let broadcast = Broadcast::new();

    let aggregator = AccountsStateActor::new(broadcast.clone()).spawn();

    let manager = AccountManagerActor::new(0, broadcast).spawn();
    let shard = AccountShardActor::new(vec![manager.clone()]).spawn();

    crate::csv::process(shard, args.input).await;

    let _response = aggregator
        .send_async(AggregatorRequests::Call(Box::new(print_accounts_state)))
        .await;
}
