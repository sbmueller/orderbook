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
            .expect(&format!("Could not open file {}", &args.file.display()))
    });
    while let Ok(data) = order_receiver.recv() {
        order_book.add_order(data);
    }
    // Drop order_book here to implicitly destroy output_sender and let the output thread terminate
    std::mem::drop(order_book);
    let output_handle = thread::spawn(move || {
        while let Ok(data) = output_receiver.recv() {
            println!("{}", data);
        }
    });
    output_handle.join().unwrap();
    read_handle.join().unwrap();
}

fn process_input_orders(
    path: &PathBuf,
    sender: Sender<order_book::order::Order>,
) -> Result<(), csv::Error> {
    let mut rdr = csv::ReaderBuilder::new()
        .comment(Some(b'#'))
        .has_headers(false)
        .flexible(true)
        .trim(csv::Trim::All)
        .from_path(path)?;
    for result in rdr.records() {
        let record = result?;
        sender
            .send(order_book::order::Order::from(&record))
            .unwrap();
    }
    Ok(())
}
