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

// fn read_transactions(filename: &str) -> Result<Vec<Transaction>, Box<dyn Error>> {
//     println!("in read-transactions");
//     let file = File::open(filename)?;
//     let mut rdr = csv::ReaderBuilder::new().trim(Trim::All).from_reader(file);
//     let headers = rdr.headers()?;
//     println!("headers: {:?}", headers);
//     let mut transactions = Vec::new();
//     for result in rdr.deserialize() {
//         println!("in for loop");
//         println!("result: {:?}", result);
//         let record: Transaction = result?;
//         transactions.push(record);
//     }
//     println!("reached end of read transactions");
//     Ok(transactions)
// }

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
    //output_file_path: String
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

    #[test]
    fn one_result() {
        let args = vec![String::from("file.txt")];
        let config = Config::build(&args).unwrap();
        assert_eq!(config.file_path, "file.txt");
    }
}
