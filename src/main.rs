use clap::Parser;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};
use std::thread;

mod order_book;

/// CLI tool that implements an order book for a given input file
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Trade mode, provided then orders that cross the book will be traded instead of rejected
    #[clap(short, long, value_parser, default_value_t = false)]
    trade: bool,
    /// Path to a input CSV file
    #[clap(parse(from_os_str))]
    file: PathBuf,
}

fn main() {
    let args = Args::parse();
    let (order_sender, order_receiver) = channel();
    let (output_sender, output_receiver) = channel();
    let mut order_book = order_book::OrderBook::new(output_sender, args.trade);
    let read_handle = thread::spawn(move || {
        process_input_orders(&args.file, order_sender)
            .unwrap_or_else(|_| panic!("Could not open file {}", &args.file.display()))
    });
    while let Ok(data) = order_receiver.recv() {
        order_book.add_order(data);
    }
    let output_handle = thread::spawn(move || {
        while let Ok(data) = output_receiver.recv() {
            println!("{}", data);
        }
    });
    // Drop order_book here to implicitly destroy output_sender and let the output thread terminate
    std::mem::drop(order_book);
    output_handle.join().unwrap();
    read_handle.join().unwrap();
}

/// Read orders from a provided CSV file and send the content as `order::Order` to another thread,
/// using the provided `sender`.
///
/// # Args
/// * `path`: Handle for a CSV file containing orders
/// * `sender`: MPSC sender to use for communicating orders
///
/// # Return
/// A `Result` containing a `unit` or an `csv::Error`, if there is an issue with reading the
/// provided CSV file.
fn process_input_orders(
    path: &PathBuf,
    sender: Sender<order_book::order::Order>,
) -> Result<(), csv::Error> {
    let mut reader = csv::ReaderBuilder::new()
        .comment(Some(b'#'))
        .has_headers(false)
        .flexible(true)
        .trim(csv::Trim::All)
        .from_path(path)?;
    for result in reader.records() {
        let record = result?;
        sender
            .send(order_book::order::Order::from(&record))
            .unwrap();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orders_without_trading() {
        let input = "\
N, 1, IBM, 10, 100, B, 1
N, 1, IBM, 12, 100, S, 2
N, 2, IBM, 9, 100, B, 101
N, 2, IBM, 11, 100, S, 102
N, 1, IBM, 11, 100, B, 3
N, 2, IBM, 10, 100, S, 103
N, 1, IBM, 10, 100, B, 4
N, 2, IBM, 11, 100, S, 104
F

N, 1, AAPL, 10, 100, B, 1
N, 1, AAPL, 12, 100, S, 2
N, 2, AAPL, 11, 100, S, 102
N, 2, AAPL, 10, 100, S, 103
N, 1, AAPL, 10, 100, B, 3
F

N, 1, VAL, 10, 100, B, 1
N, 2, VAL, 9, 100, B, 101
N, 2, VAL, 11, 100, S, 102
N, 1, VAL, 11, 100, B, 2
N, 2, VAL, 11, 100, S, 103
F

N, 1, IBM, 10, 100, B, 1
N, 1, IBM, 12, 100, S, 2
N, 2, IBM, 9, 100, B, 101
N, 2, IBM, 11, 100, S, 102
N, 2, IBM, 9, 100, S, 103
F

N, 1, IBM, 10, 100, B, 1
N, 1, IBM, 12, 100, S, 2
N, 2, IBM, 9, 100, B, 101
N, 2, IBM, 11, 100, S, 102
N, 1, IBM, 12, 100, B, 103
F

N, 1, IBM, 10, 100, B, 1
N, 1, IBM, 16, 100, S, 2
N, 2, IBM, 9, 100, B, 101
N, 2, IBM, 15, 100, S, 102

N, 2, IBM, 11, 100, B, 103
N, 1, IBM, 14, 100, S, 3
F

N, 1, IBM, 10, 100, B, 1
N, 1, IBM, 12, 100, S, 2
N, 2, IBM, 9, 100, B, 101
N, 2, IBM, 11, 100, S, 102
N, 2, IBM, 10, 20, S, 103
F

N, 1, IBM, 10, 100, B, 1
N, 1, IBM, 12, 100, S, 2
N, 2, IBM, 9, 100, B, 101
N, 2, IBM, 11, 100, S, 102
N, 1, IBM, 11, 20, B, 3
F

N, 1, IBM, 10, 100, B, 1
N, 1, IBM, 12, 100, S, 2
N, 2, IBM, 9, 100, B, 101
N, 2, IBM, 11, 100, S, 102
C, 1, 1
C, 2, 102
F

N, 1, IBM, 10, 100, B, 1
N, 1, IBM, 12, 100, S, 2
N, 2, IBM, 9, 100, B, 101
N, 2, IBM, 11, 100, S, 102
C, 1, 2
C, 2, 101
F

N, 1, IBM, 10, 100, B, 1
N, 1, IBM, 12, 100, S, 2
N, 2, IBM, 9, 100, B, 101
N, 2, IBM, 11, 100, S, 102
C, 1, 1
C, 2, 101
F

N, 1, IBM, 10, 100, B, 1
N, 1, IBM, 12, 100, S, 2
N, 2, IBM, 9, 100, B, 101
N, 2, IBM, 11, 100, S, 102
N, 2, IBM, 11, 100, S, 103
C, 2, 103
C, 2, 102
C, 1, 2
F
";
        let output = "\
A, 1, 1
B, B, 10, 100
A, 1, 2
B, S, 12, 100
A, 2, 101
A, 2, 102
B, S, 11, 100
R, 1, 3
R, 2, 103
A, 1, 4
B, B, 10, 200
A, 2, 104
B, S, 11, 200

A, 1, 1
B, B, 10, 100
A, 1, 2
B, S, 12, 100
A, 2, 102
B, S, 11, 100
R, 2, 103
A, 1, 3
B, B, 10, 200

A, 1, 1
B, B, 10, 100
A, 2, 101
A, 2, 102
B, S, 11, 100
R, 1, 2
A, 2, 103
B, S, 11, 200

A, 1, 1
B, B, 10, 100
A, 1, 2
B, S, 12, 100
A, 2, 101
A, 2, 102
B, S, 11, 100
R, 2, 103

A, 1, 1
B, B, 10, 100
A, 1, 2
B, S, 12, 100
A, 2, 101
A, 2, 102
B, S, 11, 100
R, 1, 103

A, 1, 1
B, B, 10, 100
A, 1, 2
B, S, 16, 100
A, 2, 101
A, 2, 102
B, S, 15, 100
A, 2, 103
B, B, 11, 100
A, 1, 3
B, S, 14, 100

A, 1, 1
B, B, 10, 100
A, 1, 2
B, S, 12, 100
A, 2, 101
A, 2, 102
B, S, 11, 100
R, 2, 103

A, 1, 1
B, B, 10, 100
A, 1, 2
B, S, 12, 100
A, 2, 101
A, 2, 102
B, S, 11, 100
R, 1, 3

A, 1, 1
B, B, 10, 100
A, 1, 2
B, S, 12, 100
A, 2, 101
A, 2, 102
B, S, 11, 100
A, 1, 1
B, B, 9, 100
A, 2, 102
B, S, 12, 100

A, 1, 1
B, B, 10, 100
A, 1, 2
B, S, 12, 100
A, 2, 101
A, 2, 102
B, S, 11, 100
A, 1, 2
A, 2, 101

A, 1, 1
B, B, 10, 100
A, 1, 2
B, S, 12, 100
A, 2, 101
A, 2, 102
B, S, 11, 100
A, 1, 1
B, B, 9, 100
A, 2, 101
B, B, -, -

A, 1, 1
B, B, 10, 100
A, 1, 2
B, S, 12, 100
A, 2, 101
A, 2, 102
B, S, 11, 100
A, 2, 103
B, S, 11, 200
A, 2, 103
B, S, 11, 100
A, 2, 102
B, S, 12, 100
A, 1, 2
B, S, -, -

";
        let result = process_and_return_output(input, false);
        assert_eq!(result, output)
    }

    #[test]
    fn test_orders_with_trading() {
        let input = "\
N, 1, IBM, 10, 100, B, 1
N, 1, IBM, 12, 100, S, 2
N, 2, IBM, 9, 100, B, 101
N, 2, IBM, 11, 100, S, 102
N, 1, IBM, 12, 100, B, 103
F

N, 1, VAL, 10, 100, B, 1
N, 2, VAL, 9, 100, B, 101
N, 2, VAL, 11, 100, S, 102
N, 1, VAL, 11, 100, B, 2
N, 2, VAL, 11, 100, S, 103
F
";

        let output = "\
A, 1, 1
B, B, 10, 100
A, 1, 2
B, S, 12, 100
A, 2, 101
A, 2, 102
B, S, 11, 100
A, 1, 103
T, 1, 103, 2, 102, 11, 100
B, S, 12, 100

A, 1, 1
B, B, 10, 100
A, 2, 101
A, 2, 102
B, S, 11, 100
A, 1, 2
T, 1, 2, 2, 102, 11, 100
B, S, -, -
A, 2, 103
B, S, 11, 100

";
        let result = process_and_return_output(input, true);
        assert_eq!(result, output)
    }

    fn process_and_return_output(input: &str, trading: bool) -> String {
        let (output_sender, output_receiver) = channel();
        let mut order_book = order_book::OrderBook::new(output_sender, trading);
        let mut reader = csv::ReaderBuilder::new()
            .comment(Some(b'#'))
            .has_headers(false)
            .flexible(true)
            .trim(csv::Trim::All)
            .from_reader(input.as_bytes());
        for result in reader.records() {
            let record = result.unwrap();
            order_book.add_order(order_book::order::Order::from(&record));
        }
        let output_handle = thread::spawn(move || -> String {
            let mut result = String::new();
            while let Ok(data) = output_receiver.recv() {
                result += &data;
                result += "\n";
                println!("{data}");
            }
            result
        });
        std::mem::drop(order_book);
        output_handle.join().unwrap()
    }
}
