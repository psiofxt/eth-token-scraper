extern crate reqwest;
extern crate env_logger;
extern crate url;
extern crate serde_json;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate base64;

use std::thread;
use std::time::Duration;
use std::env;
use std::collections::HashSet;

use url::Url;

use serde_json::{Value};


#[derive(Serialize, Deserialize, Debug)]
struct LogResponse {
    id: i32,
    jsonrpc: String,
    result: Vec<Value>
}


#[derive(Serialize, Deserialize, Debug)]
struct BalanceResponse {
    id: i32,
    jsonrpc: String,
    result: String
}


fn get_balance(addr: String) -> Result<BalanceResponse, Box<std::error::Error>> {
    let parsed_adder = format!(
        "0x{}", &addr[27..67]
    );
    let params = format!(
        "params=[\
            \"{}\",\
            \"{}\"\
        ]",
        parsed_adder,
        "latest"
    );
    let url = format!("https://api.infura.io/v1/jsonrpc/{}/eth_getBalance?{}",
                       env::var("ETH_NETWORK").unwrap(), params);
    let url = Url::parse(&url)?;
    let mut res = reqwest::get(url)?;
    let text = res.text()?;
    let v: BalanceResponse = match serde_json::from_str(&text){
        Result::Ok(val) => {val},
        Result::Err(err) => {panic!("Unable to parse json: {}",err)}
    };

    println!("{:?}", v.result);
    Ok(v)
}


fn get_logs() -> Result<LogResponse, Box<std::error::Error>> {
    let params = format!(
        "params=[\
            {{\
                \"address\": \"{}\",\
                \"fromBlock\": \"{}\",\
                \"toBlock\": \"{}\",\
                \"topics\": [\"{}\"]\
            }}\
        ]",
        env::var("CONTRACT_ADDRESS").unwrap(),
        "0x60998B",
        "latest",
        "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
    );
    let url = format!("https://api.infura.io/v1/jsonrpc/{}/eth_getLogs?{}",
                      env::var("ETH_NETWORK").unwrap(), params);
    let url = Url::parse(&url)?;

    let mut res = reqwest::get(url)?;
    let text = res.text()?;
    let v: LogResponse = match serde_json::from_str(&text){
        Result::Ok(val) => {val},
        Result::Err(err) => {panic!("Unable to parse json: {}",err)}
    };
    Ok(v)
}


fn main() {
    let mut thread_vector = vec![];
    let logs = get_logs().unwrap();
    let result: Vec<Value> = logs.result;
    let mut addresses: HashSet<String> = HashSet::new();
    for entry in result {
        addresses.insert(entry["topics"][2].to_string());
    }
    for address in addresses {
        thread_vector.push(
            thread::spawn(
                move || { get_balance(address); thread::sleep(Duration::from_millis(1)); }
            )
        );
    }
    for child in thread_vector {
       match child.join() {
          Ok(_) => (),
          Err(why) => println!("Join failure {:?}", why),
       };
   }
}
