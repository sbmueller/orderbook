# Orderbook

Coding task performed June 2022 in context of an interview process. There was a
48h time limit to complete the task.

Copyright (C) 2022 Sebastian Müller

## Task

The task was to implement an order book that maintains orders at a asset
exchange. The input of the application is a CSV file with either new orders,
order cancellations or book flushes in the format

```
# New order
N, user_id, symbol, price, quantity, side, order_id
# Cancellation
C, user_id, order_id
# Flush
F
```

This input file is read in a separate thread while processing of the orders
takes place in the main thread. Orders that cross the book are rejected by
default, but can be matched and traded with the `--trade` flag. The application
outputs a log on a separate thread with acknowledgements, top-of-book changes,
rejection of orders or trades of orders in the format

```
# Acknowledgement
A, user_id, order_id
# Top-of-book change
B, side, price, quantity
# Reject
R, user_id, order_id
# Trade
T, user_id_buyer, order_id_buyer, user_id_seller, order_id_seller, price, quantity
```

## Usage

The project compiles to a command line tool. Use
```
cargo build --release
```
to compile the program. The default output directory is `target/release`. In
there, call the executable with the `--help` flag to get usage information
```
./target/release/orderbook --help
```

## Assumptions Taken

- Input files are always in the correct format
- Orders are sane (e.g. cancellation can only be done for existing orders)
- The application only works for inputs that address the same trading symbol.
  For different symbols, the inputs need to be separated and the application
  can be executed in multiple instances for each symbol.
- Trades are only performed as-whole. There are no partial trades.

## Design Decisions

The heart of the application is the order book, which is implemented by two
`BTreeMap`s for the asks and the bids, using price as key and a vector of
orders as value. This is inspired by [bigfatwhale](https://github.com/bigfatwhale/orderbook/tree/master/rust).
Since this data structure is ordered and implements `DoubleEndedIterator` it is
easy and fast to identify the highest bid and the lowest ask. These values need
to be determined frequently to match trades in a real application. The vectors
in the values of the `BTreeMap` contain orders sorted by time, so the 2nd
priority besides price can be considered as well.

There is one thread for reading the input file. The thread uses a MPSC channel
to send interpreted orders to the main thread. A second thread is responsible
for the logging, where the same design is applied. The main thread sends
messages to be published to the logging thread.

## Time and Space Complexities

Using a `BTreeMap`, the time complexity of adding orders is always O(log n).
Finding the lowest bid and highest ask takes O(1). Cancelling an order will
take O(n) time since a linear search is applied to find the desired order.
Matching a new order takes O(k) time. n is the number of total orders in the
order book while k are the number of orders on the highest bid or lowest ask.

The space complexity for a BTree is defined as O(n) however I'm not sure if
that also applies for a `BTreeMap`.

## Outlook (if more time was available)

### Improve error handling
- Handle malfomed input files
- Handle insane orders (e.g. cancel of non-existing order)

### Unit Tests

No tests have been written for this solution due to the time limit. However the
code is designed to be easily testable. The `OrderBook` takes a `mpsc::Sender`
as argument in its factory function and using dependency insertion, a test case
could insert the sender for a test purpose channel and monitor the output of
the `OrderBook`. Then, the inputs from the provided example CSV can be
converted to `Order`s in the test cases and passed into the `OrderBook`. The
`mpsc::Receiver` can then check if the correct output is produced for each
input.
