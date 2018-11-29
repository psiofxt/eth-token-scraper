extern crate reqwest;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate url;
extern crate serde_json;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate base64;
extern crate mongodb;

use mongodb::{bson, doc};
use mongodb::{Client, ThreadedClient};
use mongodb::db::ThreadedDatabase;
use mongodb::coll::options::UpdateOptions;

use std::thread;
use std::time::Duration;
use std::env;
use std::collections::HashSet;
use std::cmp::Ordering;

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

static BALANCE_SHA3: &str = "0x70a08231000000000000000000000000";
const UPDATE_OPTIONS: Option<UpdateOptions> = Some(
    UpdateOptions {upsert: Some(true), write_concern: None});


fn get_balance(address: Vec<String>) -> Result<(), Box<std::error::Error>> {
    for addr in address {
        let parsed_adder = format!(
            "{}{}", BALANCE_SHA3, &addr[27..67]
        );
        let stored_addr = format!(
            "0x{}", &addr[27..67]
        );
        let params = format!(
            "params=[\
                {{\
                    \"to\": \"{}\",\
                    \"data\": \"{}\"\
                }},\
                \"latest\"\
            ]",
            env::var("CONTRACT_ADDRESS").unwrap(),
            parsed_adder
        );
        let url = format!("https://api.infura.io/v1/jsonrpc/{}/eth_call?{}",
                           env::var("ETH_NETWORK").unwrap(), params);
        let url = Url::parse(&url)?;
        let mut res = reqwest::get(url)?;
        let text = res.text()?;
        let v: BalanceResponse = match serde_json::from_str(&text){
            Result::Ok(val) => {val},
            Result::Err(err) => {panic!("Unable to parse json: {}",err)}
        };
        let result: String = v.result;
        if result == "0x0000000000000000000000000000000000000000000000000000000000000000" {
            continue
        };
        let client = Client::connect("localhost", 27017)
            .expect("Failed to initialize standalone client.");

        let coll = client.db(&env::var("MONGO_DB_NAME").unwrap()).collection("addresses");

        let filter_doc = doc! {
            "address": stored_addr,
        };
        let update_doc = doc! {
            "$set" => {"balance": result}
        };

        coll.update_one(filter_doc, update_doc, UPDATE_OPTIONS)
            .ok().expect("Failed to insert document.");

    }
    Ok(())
}


fn get_logs(from_block: String) -> Result<LogResponse, Box<std::error::Error>> {
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
        from_block,
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
    /*
        Connect to mongo and retrieve the last block used.
    */
    let client = Client::connect("localhost", 27017)
        .expect("Failed to initialize standalone client.");
    let coll = client.db(&env::var("MONGO_DB_NAME").unwrap()).collection("last_block");
    let cursor = coll.find_one(None, None).unwrap();
    let doc = cursor.expect("None found");
    let from_block: String = doc.get("last_block")
        .unwrap()
        .to_string()
        .replace("\"", "");
    info!("Using block: {:?}", from_block);

    /*
        Process the events in the contract and retrieve the block of the last event.
        Then update the new latest block in the db.
    */
    let logs = get_logs(from_block.clone()).unwrap();
    let result: Vec<Value> = logs.result;
    let mut addresses: HashSet<String> = HashSet::new();
    let latest_block = result[result.len() - 1]["blockNumber"].to_string();
    let filter_doc = doc! {
        "last_block": from_block.clone(),
    };
    let update_doc = doc! {
        "$set" => {"last_block": latest_block}
    };
    coll.update_one(filter_doc, update_doc, UPDATE_OPTIONS)
        .ok().expect("Failed to insert document.");

    /*
        Convert the HashSet into a Vector of unique addresses (token holders).
        At this point, addresses can contain a 0x0 balance so the length of
        the vector will not be accurate.
    */
    for entry in result {
        addresses.insert(entry["topics"][2].to_string());
    }
    let mut address_vec: Vec<_> = addresses.drain().collect();

    /*
        Create a vector to store the spawned threads. Threads are chunked into groups of 5
        addresses by default. This value may need to change depending on amount of addresses
        vector or operating system restrictions.
    */
    let mut thread_vector = vec![];
    while !address_vec.is_empty() {
        let end = match 3.cmp(&address_vec.len()) {
            Ordering::Less => 3,
            Ordering::Equal => 3,
            _ => address_vec.len()
        };
        let u: Vec<_> = address_vec.drain(0..end).collect();
        thread_vector.push(
            thread::spawn(
                move || {
                    get_balance(u).expect("Get balance failed");
                    thread::sleep(Duration::from_millis(1));
                }
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
