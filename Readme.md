
# Descripiton

I will try to approach this demo as I like to solve problems: in three phases
1 - Do the minimum to work;
2 - Treat edge cases and give maturity;
3 - Improve performance;

I will also implement the problem in the following order:

1 - Accounts crate (simple operations)
2 - Aggregated result
3 - CLI
4 - Benchmark
5 - Rest of the operations

We should be Complete and Correct here.

6 - Stream CSV
7 - Multi-CSV using multiple threads

# Architecture

Certainly to solve the CLI problem, we don't need a lot of architecture. Just read the CSV, aggregate the data, print it and done! 

To be a little bit more realistic, I will implement this demo as something more complex.

I was torn about doing this from a data analysis or from a multi-threaded server perspective. I decide to mix and match both in the end.

The CLI will act as a client. It will read the CSV and "call the server". Everything will happen in the same process, but components will be slit apart using channels, to make it easy to phisically split them.

The server will be actor based. I could use https://github.com/bastion-rs/bastion which is a copy from Akka, but to avoid too
much complexity and to be able to actually build something, I will do a "poor-man" actor framework using https://github.com/tokio-rs/tokio
directly. First, because the role asked for tokio knowledge; and second because will be more fun to present.

I could also very easily allocate all accounts in memory. But the pdf hints about performance in extreme cases. If we decide to test this with u32::max accounts, this alone will use a LOT of memory: 4GB per byte needed to serve each account. Doable to host this in one server, but very easy to loose control and end up needing +128GB.

To avoid this I will complicate a little bit, but hopefully will be interesting. Accounts will live behind AccountManagers. These will be sharded.

So the CLI will actually call a ShardedAccountManager, that will delegate to an AccountManager, that will delegate to the Account actor. Akka/Bastion would help here. But I hope the code will end up readable.

This alone would suffice to have a scalable solution. I will go one step further that I will implement what some actor frameworks call "virtual actors". The actor will not live in memory 100% of the time. I will save and load them using a LRU and RocksDB. This way we can scale horizontally, but also control memory usage of each node.

This will keep only the "current state" of the system. The result I want to give from a data analysis architecture.

So each actor will raise an event when its state is updated. I will "broadcast" them to another actor that will aggregate results. Again these results may not fit in memory and we need to be careful.

In the real world, we could use a columnar database. Here I will use https://github.com/pola-rs/polars taht will simulate this using Apache Arrow. Accounts will be sharded again in megabytes of data chunks and persisted.

After everything is finished I will print the result and clean everything.

# Testing

I like "property based testing" a lot. Some operations on the account have the looks of do-undo, which is perfect for property based testing.
In Rust I like using 

# Money

Money seems a pretty important type for a bank. Would make totally sense to have a "money" crate to deal with money. I will use https://github.com/paupino/rust-decimal to implement a very simple money struct.

# Out of Order

Process out of order is very complex. If we are aiming for 100% we can implement the accounts actor with event sourcing. It persist all commands sorted by transaction order. So we can insert a old command and everything will still make sense in the end.

Problems are:
1 - If we need to have a "current view" of the system. We may be seeing a problematic view, that will *eventually* be corrected;
2 - Is almost impossible to persist the whole history forever. We will need to "checkpoint" the state as some point. This will put a limit on how late a command can arrive.

There is one specific test to test out or order. Real system would need much more, of course.
## Example of Out of Order

### Chargeback before Dispute

..
ChargebackRequest(ChargebackRequest { account_id: 1, transaction_id: 1 })
DisputeRequest(DisputeRequest { account_id: 1, transaction_id: 1 })
..
client,available,held,total,locked
1,0,0,0,true
2,2,0,2,false

In this example the Chargeback arrives before Dispute.

###  Withdraw before Deposit

...
WithdrawRequest(WithdrawRequest { account_id: 2, transaction_id: 5, amount: Money { amount: 3, currency: Bitcoin } })
DepositRequest(DepositRequest { account_id: 2, transaction_id: 2, amount: Money { amount: 2, currency: Bitcoin } })
...
client,available,held,total,locked
1,0,0,0,true
2,2,0,2,false