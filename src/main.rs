use std::collections::HashMap;
use std::env;
use std::error::Error;

use std::process;

use account::Account;
use account::Transaction;
use csv_async::Trim;
use std::sync::Arc;
use tokio::fs::File;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
mod account;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    let config = Config::build(&args).unwrap_or_else(|err| {
        println!("Problem parsing arguments: {err}");
        process::exit(1);
    });
    let accounts = Arc::new(Mutex::new(HashMap::new()));
    let deposits = Arc::new(Mutex::new(HashMap::new()));
    process_transactions(&config.file_path, accounts.clone(), deposits.clone()).await?;
    output_accounts(accounts.clone()).await?;

    Ok(())
}

async fn process_transactions(
    file: &str,
    accounts: Arc<Mutex<HashMap<u16, Account>>>,
    deposits: Arc<Mutex<HashMap<u32, (u16, f64)>>>,
) -> Result<(), Box<dyn Error>> {
    let mut csv_reader = csv_async::AsyncReaderBuilder::new()
        .trim(Trim::All)
        .create_deserializer(File::open(file).await?);
    while let Some(record) = csv_reader.deserialize::<Transaction>().next().await {
        let transaction = record?;
        let mut accounts = accounts.lock().await;
        let mut deposits = deposits.lock().await;
        let account = accounts
            .entry(transaction.client)
            .or_insert_with(Account::new);

        if account.locked {
            continue;
        }

        match transaction.kind.as_str() {
            "deposit" => {
                if let Some(amount) = transaction.amount {
                    account.deposit(amount);
                    deposits.insert(transaction.tx, (transaction.client, amount));
                }
            }
            "withdrawal" => {
                if let Some(amount) = transaction.amount {
                    account.withdrawal(amount);
                }
            }
            "dispute" => {
                if let Some((client, amount)) = deposits.get(&transaction.tx) {
                    if *client == transaction.client {
                        account.dispute(*amount);
                    }
                }
            }
            "resolve" => {
                if let Some((client, amount)) = deposits.get(&transaction.tx) {
                    if *client == transaction.client {
                        account.resolve(*amount);
                        deposits.remove(&transaction.tx);
                    }
                }
            }
            "chargeback" => {
                if let Some((client, amount)) = deposits.get(&transaction.tx) {
                    if *client == transaction.client {
                        account.chargeback(*amount);
                        deposits.remove(&transaction.tx);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

async fn output_accounts(accounts: Arc<Mutex<HashMap<u16, Account>>>) -> csv_async::Result<()> {
    let accounts = accounts.lock().await;
    let mut wtr = csv_async::AsyncWriter::from_writer(tokio::io::stdout());
    wtr.write_record(&["client", "available", "held", "total", "locked"])
        .await?;
    for (client, account) in accounts.iter() {
        wtr.write_record(&[
            client.to_string(),
            format!("{:.4}", account.available),
            format!("{:.4}", account.held),
            format!("{:.4}", account.total),
            account.locked.to_string(),
        ])
        .await?;
    }
    wtr.flush().await?;
    Ok(())
}

struct Config {
    file_path: String,
}

impl Config {
    fn build(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 2 {
            return Err("not enough arguments");
        }

        let file_path = args[1].clone();

        Ok(Config { file_path })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tokio::sync::Mutex;

    #[tokio::test]
    /// Tests the deposit transaction logic.
    async fn test_deposit_transaction() {
        // Initialize test data
        let accounts = HashMap::new();
        let accounts_mutex = Arc::new(Mutex::new(accounts));
        let deposits = Arc::new(Mutex::new(HashMap::new()));

        // Create a sample deposit transaction
        let transaction = Transaction {
            kind: "deposit".to_string(),
            client: 1,
            tx: 1,
            amount: Some(100.0),
        };

        // Process the transaction
        {
            let mut accounts = accounts_mutex.lock().await;
            let mut deposits = deposits.lock().await;
            let account = accounts
                .entry(transaction.client)
                .or_insert_with(Account::new);
            account.deposit(transaction.amount.unwrap());
            deposits.insert(
                transaction.tx,
                (transaction.client, transaction.amount.unwrap()),
            );
        }

        // Verify the account state after the transaction
        let accounts = accounts_mutex.lock().await;
        let account = accounts.get(&transaction.client).unwrap();
        assert_eq!(account.available, 100.0);
        assert_eq!(account.held, 0.0);
        assert_eq!(account.total, 100.0);
        assert_eq!(account.locked, false);
    }

    #[tokio::test]
    async fn test_withdrawal_transaction() {
        // Initialize test data
        let accounts = HashMap::new();
        let accounts_mutex = Arc::new(Mutex::new(accounts));

        // Create a sample account with initial balance
        {
            let mut accounts = accounts_mutex.lock().await;
            let account = accounts.entry(1).or_insert_with(Account::new);
            account.deposit(100.0);
        }

        // Create a sample withdrawal transaction
        let transaction = Transaction {
            kind: "withdrawal".to_string(),
            client: 1,
            tx: 2,
            amount: Some(50.0),
        };

        // Process the transaction
        {
            let mut accounts = accounts_mutex.lock().await;
            let account = accounts.get_mut(&transaction.client).unwrap();
            let success = account.withdrawal(transaction.amount.unwrap());
            assert!(success);
        }

        // Verify the account state after the transaction
        let accounts = accounts_mutex.lock().await;
        let account = accounts.get(&transaction.client).unwrap();
        assert_eq!(account.available, 50.0);
        assert_eq!(account.held, 0.0);
        assert_eq!(account.total, 50.0);
        assert_eq!(account.locked, false);
    }

    #[tokio::test]
    async fn test_dispute_transaction() {
        // Initialize test data
        let accounts = HashMap::new();
        let accounts_mutex = Arc::new(Mutex::new(accounts));
        let deposits = Arc::new(Mutex::new(HashMap::new()));

        // Create a sample account with initial balance and a deposit
        {
            let mut accounts = accounts_mutex.lock().await;
            let mut deposits = deposits.lock().await;
            let account = accounts.entry(1).or_insert_with(Account::new);
            account.deposit(100.0);
            deposits.insert(1, (1, 100.0));
        }

        // Create a sample dispute transaction
        let transaction = Transaction {
            kind: "dispute".to_string(),
            client: 1,
            tx: 1,
            amount: None,
        };

        // Process the dispute transaction
        {
            let mut accounts = accounts_mutex.lock().await;
            let deposits = deposits.lock().await;
            let account = accounts.get_mut(&transaction.client).unwrap();
            account.dispute(100.0); // Dispute the entire deposit amount
            let (client, amount) = deposits.get(&transaction.tx).unwrap();
            assert_eq!(*client, transaction.client);
            assert_eq!(*amount, 100.0); // Ensure the deposit is marked as disputed
        }

        // Verify the account state after the dispute transaction
        let accounts = accounts_mutex.lock().await;
        let account = accounts.get(&transaction.client).unwrap();
        assert_eq!(account.available, 0.0);
        assert_eq!(account.held, 100.0);
        assert_eq!(account.total, 100.0);
        assert_eq!(account.locked, false);
    }

    #[tokio::test]
    async fn test_chargeback_transaction() {
        // Initialize test data
        let accounts = HashMap::new();
        let accounts_mutex = Arc::new(Mutex::new(accounts));
        let deposits = Arc::new(Mutex::new(HashMap::new()));

        // Create a sample account with initial balance and a disputed deposit
        {
            let mut accounts = accounts_mutex.lock().await;
            let mut deposits = deposits.lock().await;
            let account = accounts.entry(1).or_insert_with(Account::new);
            account.deposit(100.0);
            account.dispute(100.0);
            deposits.insert(1, (1, 100.0));
        }

        // Create a sample chargeback transaction
        let transaction = Transaction {
            kind: "chargeback".to_string(),
            client: 1,
            tx: 1,
            amount: None,
        };

        // Process the chargeback transaction
        {
            let mut accounts = accounts_mutex.lock().await;
            let mut deposits = deposits.lock().await;
            let account = accounts.get_mut(&transaction.client).unwrap();
            account.chargeback(100.0); // Chargeback the entire disputed amount
            deposits.remove(&transaction.tx); // Remove the disputed transaction from deposits
        }

        // Verify the account state after the chargeback transaction
        let accounts = accounts_mutex.lock().await;
        let account = accounts.get(&transaction.client).unwrap();
        assert_eq!(account.available, 0.0);
        assert_eq!(account.held, 0.0);
        assert_eq!(account.total, 0.0); // Account should be empty after chargeback
        assert_eq!(account.locked, true); // Account should be locked after chargeback
    }
}
