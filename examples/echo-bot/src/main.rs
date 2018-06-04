extern crate beryllium;
extern crate chrono;
extern crate env_logger;
extern crate log;
extern crate regex;
extern crate hyper;
#[macro_use]extern crate lazy_static;

use beryllium::{BotClient, BotService, Handler, EventData, Event, ReqType};
use chrono::offset::Utc;
use env_logger::LogBuilder;
use log::{LogRecord, LogLevelFilter};
use std::env;
//use std::fs;
//use std::path::PathBuf;
use regex::Regex;
use hyper::Method;


enum CexRoute{
    Null,
    Ticker,
    Chart,
    Candle,

}

fn some_helper_function(text: &str, route: &mut CexRoute) -> bool {
    lazy_static! {
        static ref RETICKER: Regex = Regex::new("比特币行情").unwrap();
        static ref RECHART: Regex = Regex::new("比特币K行情").unwrap();
        static ref REHCHART: Regex = Regex::new("比特币candle行情").unwrap();
    }


    let (route0,ret)=if RETICKER.is_match(text) {
        (CexRoute::Ticker,true)
    } else if RECHART.is_match(text) {
        (CexRoute::Chart,true)
    } else if REHCHART.is_match(text) {
        (CexRoute::Candle,true)
    } else {
        (CexRoute::Null,false)
    };

    *route=route0;

    return ret;
}



pub struct EchoServer;

impl Handler for EchoServer {
    fn handle(&self, data: EventData, client: BotClient) {
        match data.event {
            Event::Message { ref text, ref from } => {
                println!("{} received message from {}", data.bot_id, from);
                let mut r=CexRoute::Null;
                if some_helper_function(&text, &mut r) {
                    let (url, method,req) = match r {
                        CexRoute::Chart => ("price_stats/BTC/USD".to_owned(), Method::Post,  Some(ReqType::VcoinTickerReq{lastHours:24,maxRespArrSize:100})),
                        CexRoute::Candle => ("ohlcv/hd/".to_owned()+"20180310"+"/BTC/USD", Method::Get, None),
                        _ => ("ticker/BTC/USD".to_owned(), Method::Get, None),
                    };
                    client.send_cex_message(url, method, req);
                }
            },
            Event::ConversationMemberJoin { ref members_joined } => {
                println!("Members joined: {:?}", members_joined);
            },
            Event::ConversationMemberLeave { ref members_left } => {
                println!("Members left: {:?}", members_left);
            },
            Event::ConversationRename => {
                println!("Conversation has been renamed to {}",
                         data.conversation.name);
            },
            _ => (),
        }
    }
}

macro_rules! get_env {
    ($var:expr, $default:expr) => {
        match env::var($var) {
            Ok(val) => {
                println!("Found {}={} in env", $var, val);
                val
            },
            Err(_) => {
                println!("Cannot find {}, using default {}", $var, $default);
                String::from($default)
            },
        }
    };
}

fn main() {
    let mut builder = LogBuilder::new();
    builder.format(|record: &LogRecord| format!("{:?}: {}: {}", Utc::now(), record.level(), record.args()))
           .filter(None, LogLevelFilter::Info);
    if let Ok(v) = env::var("RUST_LOG") {
       builder.parse(&v);
    }

    builder.init().unwrap();

    let data_path = get_env!("DB", "postgresql://han@localhost:26257/cbox"); //
    let addr = get_env!("ADDRESS", "0.0.0.0:6000").parse().unwrap();;
    let key = get_env!("KEY_PATH", "server.pem");
    let cert = get_env!("CERT_PATH", "server.crt");
    let auth = get_env!("AUTH", "0xdeadbeef");

//    if !PathBuf::from(&data_path).exists() {
//        fs::create_dir(&data_path).unwrap();
//    }

    let service = BotService::new(auth, data_path, &key, &cert).unwrap();
    service.start_listening(&addr, EchoServer);
}
