# Transaction Processing Engine

A Rust-based transaction processing engine that manages client accounts through various transaction types like deposit, withdrawal, dispute, resolve, and chargeback.

## Testing
UTs exist for each transaction type. In addition, you can run `cargo run test.csv > test_output.csv` from the root of the repository to test a real-case input file.

## Safety and Robustness
This project leverages Rust's type system to ensure memory safety and prevent common programming errors at compile-time. The use of Mutex ensures thread safety when accessing shared data structures.

## Concurrency
By reading in the input file asynchronously, we free up threads from waiting for I/O operations to complete.

## Assumptions
Transactions (each row of the input file) are handled sequentially in this project. A performance improvement would be to spawn asynchronous tasks for each transaction. I didn't do this though because I made an assumption that the order of the rows in the input file is important and that transactions must be handled in-order. One other potential optimisation here would be to have a thread-per-client. Then you could be confident that the transactions happen in-order for a given client, but get the performance benefits of concurrency.
