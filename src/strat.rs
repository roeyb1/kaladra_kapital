use std::fs::File;
use std::io::Read;
use serde::Deserialize;
use crate::CurrencyOrbType::Divine;
use crate::Price;
use crate::trade_api::{get_search_pricing, TradeSearchFilter, TradeSearchQuery, TradeStatus};

#[derive(Deserialize, Debug)]
struct ItemInputFilters {

}

#[derive(Deserialize, Debug)]
struct StratInput {
    name: Option<String>,
    term: Option<String>,
    bulk: bool,
    filters: Option<Vec<ItemInputFilters>>
}

#[derive(Deserialize, Debug)]
pub struct Strat {
    inputs: Vec<StratInput>,
    outputs: Vec<StratInput>
}

impl StratInput {
    fn search_query(&self) -> TradeSearchQuery {
        let mut filters: Vec<TradeSearchFilter> = Vec::new();
        if self.filters.is_some() {
            // #todo
            //filters.copy_from_slice(self.filters[..]);
        }
        TradeSearchQuery {
            status: TradeStatus::online,
            item_type: self.name.clone(),
            term: self.term.clone(),
            filters,
        }
    }
}

pub fn read_strat_from_file(filename: &str) -> std::io::Result<Strat> {
    let mut file = File::open(filename)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let strat: Strat = serde_json::from_str(contents.as_str()).unwrap();

    Ok(strat)
}

pub fn compute_profitability(strat: &Strat) -> Price {
    // compute the input cost:
    let mut input_cost = Price::new(0., Divine);
    for input in &strat.inputs {
        let price = get_search_pricing(input.search_query()).unwrap();
        input_cost += price;
    }

    let mut output_cost = Price::new(0., Divine);
    for output in &strat.outputs {
        let price = get_search_pricing(output.search_query()).unwrap();
        output_cost += price;
    }

    output_cost - input_cost
}