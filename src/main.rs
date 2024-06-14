use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Write;
use std::process;

use account::Account;
use account::Transaction;

mod account;
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    let config = Config::build(&args).unwrap_or_else(|err| {
        println!("Problem parsing arguments: {err}");
        process::exit(1);
    });

    let transactions = read_transactions(&args[1])?;
    let accounts = process_transactions(transactions);
    output_accounts(&accounts, &mut io::stdout())?;

    Ok(())
}

fn read_transactions(filename: &str) -> Result<Vec<Transaction>, Box<dyn Error>> {
    let file = File::open(filename)?;
    let mut rdr = csv::Reader::from_reader(file);
    let mut transactions = Vec::new();
    for result in rdr.deserialize() {
        let record: Transaction = result?;
        transactions.push(record);
    }
    Ok(transactions)
}

fn process_transactions(transactions: Vec<Transaction>) -> HashMap<u16, Account> {
    let mut accounts: HashMap<u16, Account> = HashMap::new();
    let mut disputed_transactions: HashMap<u32, (u16, f64)> = HashMap::new();

    for transaction in transactions {
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
                    disputed_transactions.insert(transaction.tx, (transaction.client, amount));
                }
            }
            "withdrawal" => {
                if let Some(amount) = transaction.amount {
                    account.withdrawal(amount);
                }
            }
            "dispute" => {
                if let Some((client, amount)) = disputed_transactions.get(&transaction.tx) {
                    if *client == transaction.client {
                        account.dispute(*amount);
                    }
                }
            }
            "resolve" => {
                if let Some((client, amount)) = disputed_transactions.get(&transaction.tx) {
                    if *client == transaction.client {
                        account.resolve(*amount);
                        disputed_transactions.remove(&transaction.tx);
                    }
                }
            }
            "chargeback" => {
                if let Some((client, amount)) = disputed_transactions.get(&transaction.tx) {
                    if *client == transaction.client {
                        account.chargeback(*amount);
                        disputed_transactions.remove(&transaction.tx);
                    }
                }
            }
            _ => {}
        }
    }

    accounts
}

fn output_accounts<W: Write>(accounts: &HashMap<u16, Account>, writer: &mut W) -> csv::Result<()> {
    let mut wtr = csv::Writer::from_writer(writer);
    wtr.write_record(&["client", "available", "held", "total", "locked"])?;
    for (client, account) in accounts {
        wtr.write_record(&[
            client.to_string(),
            format!("{:.4}", account.available),
            format!("{:.4}", account.held),
            format!("{:.4}", account.total),
            account.locked.to_string(),
        ])?;
    }
    wtr.flush()?;
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
