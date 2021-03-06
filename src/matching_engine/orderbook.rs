use std::collections::{BinaryHeap, HashMap};
use super::order::{Order, OrderKey, FillEvent, OrderSide};
use serde::{Serialize};

pub struct Orderbook {
    orders: HashMap<u64, Order>,
    best_sell_orders: BinaryHeap<OrderKey>,
    best_buy_orders: BinaryHeap<OrderKey>,
    time_counter: u64
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderbookContent {
    pub buy_orders: Vec<Order>,
    pub sell_orders: Vec<Order>
}

impl Orderbook {
    pub fn new() -> Self {
        Orderbook {
            orders: HashMap::new(),
            best_sell_orders: BinaryHeap::new(),
            best_buy_orders: BinaryHeap::new(),
            time_counter: 0
        }
    }

    pub fn process_order(&mut self, order: &mut Order)  -> Vec<FillEvent> {
        let mut match_events = Vec::new();
        let mut ids_to_remove = Vec::new();

        let best_opposite_orders = match order.order_key.order_side {
            OrderSide::Sell => &mut self.best_buy_orders,
            OrderSide::Buy => &mut self.best_sell_orders
        };

        while order.quantity != 0 {
            match best_opposite_orders.peek() {
                None => {
                    self.add_order(order);
                    break;
                }
                Some(best_opposite_order_key) => {
                    let non_match_case = match best_opposite_order_key.order_side {
                        OrderSide::Buy => order.order_key.price > best_opposite_order_key.price,
                        OrderSide::Sell => order.order_key.price < best_opposite_order_key.price
                    };
                    if non_match_case {
                        self.add_order(order);
                        break;
                    } else {
                        let best_opposite_order = self.orders.get_mut(&best_opposite_order_key.id).unwrap();
                        let fill_event = order.get_fill_event(best_opposite_order);
                        match_events.push(fill_event);

                        order.quantity -= fill_event.quantity;
                        best_opposite_order.quantity -= fill_event.quantity;

                        if best_opposite_order.empty() {
                            best_opposite_orders.pop();
                            ids_to_remove.push(best_opposite_order.order_key.id);
                        } else if best_opposite_order.quantity == 0 {
                            best_opposite_orders.pop();
                            best_opposite_order.reload_iceberg_order();
                            self.time_counter += 1;
                            best_opposite_order.order_key.timestamp = self.time_counter;
                            best_opposite_orders.push(best_opposite_order.order_key.clone());
                        }

                        if order.is_iceberg() {
                            order.reload_iceberg_order();
                        }
                    }
                }
            }
        }
        ids_to_remove.iter().for_each(|k| { self.orders.remove(&k); });
        match_events
    }


    fn add_order(&mut self, order: &mut Order) {
        self.time_counter += 1;
        order.order_key.timestamp = self.time_counter;
        self.orders.insert(order.order_key.id, *order);
        match order.order_key.order_side {
            OrderSide::Buy => self.best_buy_orders.push(order.order_key),
            OrderSide::Sell => self.best_sell_orders.push(order.order_key)
        };

    }

    fn get_buy_orders(&self) -> Vec<Order> {
        self.best_buy_orders
            .clone()
            .into_sorted_vec()
            .iter()
            .rev()
            .map(|order_key| self.orders.get(&order_key.id).unwrap().clone())
            .collect()
    }

    fn get_sell_orders(&self) -> Vec<Order> {
        self.best_sell_orders
            .clone()
            .into_sorted_vec()
            .iter()
            .rev()
            .map(|order_key| self.orders.get(&order_key.id).unwrap().clone())
            .collect()
    }

    pub fn get_orders(&self) -> OrderbookContent {
        OrderbookContent {
            buy_orders: self.get_buy_orders(),
            sell_orders: self.get_sell_orders()
        }
    }
}
