use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Transaction {
    #[serde(rename = "type")]
    pub kind: String,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<f64>,
}

#[derive(Debug)]
pub struct Account {
    pub available: f64,
    pub held: f64,
    pub total: f64,
    pub locked: bool,
}

impl Account {
    pub fn new() -> Self {
        Account {
            available: 0.0,
            held: 0.0,
            total: 0.0,
            locked: false,
        }
    }

    /// Deposits the given amount into the account.
    pub fn deposit(&mut self, amount: f64) {
        self.available += amount;
        self.total += amount;
    }

    /// Withdraws the given amount from the account if it is available.
    /// Returns true if the withdrawal was successful, false otherwise.
    pub fn withdrawal(&mut self, amount: f64) -> bool {
        if self.available >= amount {
            self.available -= amount;
            self.total -= amount;
            true
        } else {
            false
        }
    }

    /// Disputes the given amount, moving it from available to held.
    pub fn dispute(&mut self, amount: f64) {
        self.available -= amount;
        self.held += amount;
    }

    /// Resolves the given amount, moving it from held to available.
    pub fn resolve(&mut self, amount: f64) {
        self.held -= amount;
        self.available += amount;
    }

    /// Charges back the given amount, moving it from held to total and locking the account.
    pub fn chargeback(&mut self, amount: f64) {
        self.held -= amount;
        self.total -= amount;
        self.locked = true;
    }
}
