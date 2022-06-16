//! Defines the order data type

use csv::StringRecord;
static CSV_ERROR_MSG: &str = "Malformed csv! Check your input file and try again.";

/// Data structure to represent one order
pub struct Order {
    pub kind: Kind,
    pub user: i32,
    pub price: i32,
    pub qty: i32,
    pub side: Side,
    pub user_order_id: i32,
}

/// Enumeration to specify the side of the order book
pub enum Side {
    Buy,
    Sell,
}

/// Enumeration to specify the order kind
pub enum Kind {
    New,
    Cancel,
    Flush,
}

impl Order {
    /// Factory function to generate an order from a CSV StringRecord
    ///
    /// # Args
    /// - `record`: One CSV record representing one order
    ///
    /// # Return
    /// - A new `Order` representing the input data
    pub fn from(record: &StringRecord) -> Order {
        match record.get(0) {
            Some(x) => match x {
                "N" => Order::new_user_order(record),
                "C" => Order::new_cancellation(record),
                "F" => Order::new_flush(),
                &_ => panic!("{}", CSV_ERROR_MSG),
            },
            None => panic!("{}", CSV_ERROR_MSG),
        }
    }

    /// Create a new user order by interpreting the CSV record
    ///
    /// # Args
    /// - `record`: One CSV record representing a new user order
    ///
    /// # Return
    /// - A new `Order` representing the input data
    fn new_user_order(record: &StringRecord) -> Order {
        let side = match record.get(5) {
            Some(x) => match x {
                "B" => Side::Buy,
                "S" => Side::Sell,
                &_ => panic!("{}", CSV_ERROR_MSG),
            },
            None => panic!("{}", CSV_ERROR_MSG),
        };
        Order {
            kind: Kind::New,
            user: record
                .get(1)
                .expect(CSV_ERROR_MSG)
                .parse::<i32>()
                .expect(CSV_ERROR_MSG),
            price: record
                .get(3)
                .expect(CSV_ERROR_MSG)
                .parse::<i32>()
                .expect(CSV_ERROR_MSG),
            qty: record
                .get(4)
                .expect(CSV_ERROR_MSG)
                .parse::<i32>()
                .expect(CSV_ERROR_MSG),
            side,
            user_order_id: record
                .get(6)
                .expect(CSV_ERROR_MSG)
                .parse::<i32>()
                .expect(CSV_ERROR_MSG),
        }
    }

    /// Create a new cancellation order by interpreting the CSV record
    ///
    /// # Args
    /// - `record`: One CSV record representing one cancellation order
    ///
    /// # Return
    /// - A new `Order` representing the input data
    fn new_cancellation(record: &StringRecord) -> Order {
        Order {
            kind: Kind::Cancel,
            user: record
                .get(1)
                .expect(CSV_ERROR_MSG)
                .parse::<i32>()
                .expect(CSV_ERROR_MSG),
            price: 0,
            qty: 0,
            side: Side::Buy,
            user_order_id: record
                .get(2)
                .expect(CSV_ERROR_MSG)
                .parse::<i32>()
                .expect(CSV_ERROR_MSG),
        }
    }

    /// Create a new flush order
    fn new_flush() -> Order {
        Order {
            kind: Kind::Flush,
            user: 0,
            price: 0,
            qty: 0,
            side: Side::Buy,
            user_order_id: 0,
        }
    }
}
