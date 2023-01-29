//! Module that defines data structures and functions around an Orderbook.

use std::collections::BTreeMap;
use std::sync::mpsc::Sender;

pub mod order;

/// Struct to represent one order book consisting of an ask book and bid book. Every book stores a
/// collection of `Order`s for a given price value.
pub struct OrderBook {
    ask_book: BTreeMap<i32, Vec<order::Order>>,
    bid_book: BTreeMap<i32, Vec<order::Order>>,
    lowest_ask: Option<(i32, i32)>,
    highest_bid: Option<(i32, i32)>,
    log_sender: Sender<String>,
    match_orders: bool,
}

impl OrderBook {
    /// Factory function for constructing a new OrderBook
    ///
    /// # Args
    /// - `output_sender`: A mpsc sender used to send messages to the output thread
    ///
    /// # Return
    /// A new `OrderBook` instance
    pub fn new(output_sender: Sender<String>, match_orders: bool) -> OrderBook {
        OrderBook {
            ask_book: BTreeMap::new(),
            bid_book: BTreeMap::new(),
            lowest_ask: None,
            highest_bid: None,
            log_sender: output_sender,
            match_orders,
        }
    }

    /// Add an order to the order book
    ///
    /// # Args
    /// - `order`: Order to be added
    pub fn add_order(&mut self, order: order::Order) {
        match order.kind {
            order::Kind::New => self.new_user_order(order),
            order::Kind::Cancel => self.cancel_order(order),
            order::Kind::Flush => self.flush(),
        }
    }

    // The following two functions show a high amount of duplication. It could make sense to
    // refactor both into just one generalized function, either by introducing a conditional or
    // accept more arguments that determine the behavior from outside.

    /// Updates the lowest_ask member and sends a message to the output thread if a change occurred
    fn update_lowest_ask(&mut self) {
        // First bucket is also the one with the lowest price
        let lowest_bucket = self.ask_book.iter().next();
        match lowest_bucket {
            Some(bucket) => {
                let price: i32 = *bucket.0;
                // Accumulate volume over all orders in bucket
                let volume: i32 = bucket.1.iter().map(|o| o.qty).sum();
                // Check for top of book change
                if self.lowest_ask.is_none()
                    || self.lowest_ask.unwrap().0 != price
                    || self.lowest_ask.unwrap().1 != volume
                {
                    self.log_sender
                        .send(format!("B, S, {}, {}", price, volume))
                        .unwrap();
                    self.lowest_ask = Some((price, volume));
                }
            }
            // There is no ask order in the books
            None => {
                // Check if top of book was changed due to a matched order
                if self.lowest_ask.is_some() {
                    self.log_sender.send("B, S, -, -".to_string()).unwrap();
                    self.lowest_ask = None;
                }
            }
        }
    }

    /// Updates the highest_bid member and sends a message to the output thread if a change occurred
    fn update_highest_bid(&mut self) {
        // Last bucket is also the one with highest price
        let highest_bucket = self.bid_book.iter().next_back();
        match highest_bucket {
            Some(bucket) => {
                let price: i32 = *bucket.0;
                // Accumulate volume over all orders in bucket
                let volume: i32 = bucket.1.iter().map(|o| o.qty).sum();
                // Check for top of book change
                if self.highest_bid.is_none()
                    || self.highest_bid.unwrap().0 != price
                    || self.highest_bid.unwrap().1 != volume
                {
                    self.log_sender
                        .send(format!("B, B, {}, {}", price, volume))
                        .unwrap();
                    self.highest_bid = Some((price, volume));
                }
            }
            // There is no bid order in the books
            None => {
                // Check if top of book was changed due to a matched order
                if self.highest_bid.is_some() {
                    self.log_sender.send("B, B, -, -".to_string()).unwrap();
                    self.highest_bid = None;
                }
            }
        }
    }

    /// Process a new order
    ///
    /// # Args
    /// - `order`: Order to be processed
    fn new_user_order(&mut self, order: order::Order) {
        if !self.match_orders && self.crosses_the_book(&order) {
            self.log_sender
                .send(format!("R, {}, {}", &order.user, &order.user_order_id))
                .unwrap();
            return;
        }
        self.log_sender
            .send(format!("A, {}, {}", &order.user, &order.user_order_id))
            .unwrap();
        // Match orders if configured
        if self.match_orders
            && match order.side {
                order::Side::Buy => self.trade_buy_order(&order),
                order::Side::Sell => self.trade_sell_order(&order),
            }
        {
            return;
        }
        // If no matching was done, write into book
        let inserter = |book: &mut BTreeMap<i32, Vec<order::Order>>, order: order::Order| {
            let bucket = book.get_mut(&order.price);
            match bucket {
                Some(v) => v.push(order),
                None => {
                    book.insert(order.price, vec![order]);
                }
            }
        };
        match order.side {
            order::Side::Buy => {
                inserter(&mut self.bid_book, order);
                self.update_highest_bid();
            }
            order::Side::Sell => {
                inserter(&mut self.ask_book, order);
                self.update_lowest_ask();
            }
        }
    }

    /// Checks if an order would cross the book
    ///
    /// # Args
    /// - `order`: Order to be checked
    ///
    /// # Return
    /// - `true` if order would cross the book, `false` otherwise
    fn crosses_the_book(&self, order: &order::Order) -> bool {
        match order.side {
            order::Side::Buy => match self.get_lowest_ask() {
                Some(la) => order.price >= la,
                None => false,
            },
            order::Side::Sell => match self.get_highest_bid() {
                Some(hb) => order.price <= hb,
                None => false,
            },
        }
    }

    /// Try to trade buy order
    ///
    /// # Args
    /// - `buy_order`: Buy order offered to trade
    ///
    /// # Return
    /// - True if trade was performed, false otherwise
    fn trade_buy_order(&mut self, buy_order: &order::Order) -> bool {
        let mut order_traded = false;
        // TODO how to avoid key_to_be_removed?
        let mut key_to_be_removed: Option<i32> = None;
        if let Some(bucket) = self.ask_book.iter_mut().next() {
            let sell_order_pos = bucket
                .1
                .iter()
                .position(|x| x.price <= buy_order.price && x.qty == buy_order.qty);
            if let Some(pos) = sell_order_pos {
                order_traded = true;
                let sell_order = &bucket.1[pos];
                self.log_sender
                    .send(format!(
                        "T, {}, {}, {}, {}, {}, {}",
                        buy_order.user,
                        buy_order.user_order_id,
                        sell_order.user,
                        sell_order.user_order_id,
                        sell_order.price,
                        sell_order.qty
                    ))
                    .unwrap();
                bucket.1.remove(pos);
                if bucket.1.is_empty() {
                    key_to_be_removed = Some(*bucket.0);
                }
            }
        }
        if let Some(key) = key_to_be_removed {
            self.ask_book.remove(&key);
        }
        self.update_lowest_ask();
        order_traded
    }

    /// Try to trade sell order
    ///
    /// # Args
    /// - `sell_order`: Buy order offered to trade
    ///
    /// # Return
    /// - True if trade was performed, false otherwise
    fn trade_sell_order(&mut self, sell_order: &order::Order) -> bool {
        let mut order_traded = false;
        // TODO how to avoid key_to_be_removed?
        let mut key_to_be_removed: Option<i32> = None;
        if let Some(bucket) = self.bid_book.iter_mut().next_back() {
            let sell_order_pos = bucket
                .1
                .iter()
                .position(|x| x.price >= sell_order.price && x.qty == sell_order.qty);
            if let Some(pos) = sell_order_pos {
                order_traded = true;
                let buy_order = &bucket.1[pos];
                self.log_sender
                    .send(format!(
                        "T, {}, {}, {}, {}, {}, {}",
                        buy_order.user,
                        buy_order.user_order_id,
                        sell_order.user,
                        sell_order.user_order_id,
                        sell_order.price,
                        sell_order.qty
                    ))
                    .unwrap();
                bucket.1.remove(pos);
                if bucket.1.is_empty() {
                    key_to_be_removed = Some(*bucket.0);
                }
            }
        }
        if let Some(key) = key_to_be_removed {
            self.ask_book.remove(&key);
        }
        self.update_lowest_ask();
        order_traded
    }

    /// Process a cancel order
    ///
    /// # Args
    /// - `order`: Order to be processed. Is assumed to be a cancel order.
    fn cancel_order(&mut self, order: order::Order) {
        self.log_sender
            .send(format!("A, {}, {}", order.user, order.user_order_id))
            .unwrap();
        // Use closure to avoid code duplication below
        let book_remover = |book: &mut BTreeMap<i32, Vec<order::Order>>, order: &order::Order| {
            let mut key_to_be_removed: Option<i32> = None;
            for (key, value) in book.iter_mut() {
                value.retain(|o| o.user != order.user || o.user_order_id != order.user_order_id);
                if value.is_empty() {
                    key_to_be_removed = Some(*key);
                }
            }
            if let Some(key) = key_to_be_removed {
                book.remove(&key);
            }
        };
        book_remover(&mut self.ask_book, &order);
        self.update_lowest_ask();
        book_remover(&mut self.bid_book, &order);
        self.update_highest_bid();
    }

    /// Flush the order book
    fn flush(&mut self) {
        self.log_sender.send("".to_string()).unwrap();
        self.ask_book.clear();
        self.bid_book.clear();
        self.highest_bid = None;
        self.lowest_ask = None;
    }

    /// Get the price of the highest bid or None if not available
    fn get_highest_bid(&self) -> Option<i32> {
        self.highest_bid.map(|x| x.0)
    }

    /// Get the price of the lowest ask or None if not available
    fn get_lowest_ask(&self) -> Option<i32> {
        self.lowest_ask.map(|x| x.0)
    }
}
