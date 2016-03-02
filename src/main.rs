#![allow(dead_code)]
extern crate iron;
extern crate weixin;
extern crate mount;
extern crate staticfile;
extern crate router;
extern crate persistent;
extern crate mongodb;
extern crate bodyparser;
extern crate handlebars_iron as hbs;
#[macro_use] extern crate log;
extern crate env_logger;


use iron::prelude::*;
use weixin::wx;
use weixin::service;
use weixin::session;
use weixin::pay::{PrePay,pre_pay};

use weixin::model;


use mount::Mount;
use staticfile::Static;
use router::Router;
use persistent::Read as PersistRead;
use persistent::State as PersistState;
use hbs::{HandlebarsEngine, DirectorySource};

use std::path::Path;
use std::collections::HashMap;
use std::env;
use std::net::{SocketAddrV4,Ipv4Addr};
use std::error::Error;
use std::thread;

fn get_server_port() -> u16 {
    env::var("PORT").unwrap_or("80".to_string()).parse().unwrap()
}

const MAX_BODY_LENGTH: usize = 1024 * 1024 * 10;


fn main() {
    let instance = wx::WxInstance::new();
    let mut router = Router::new();
    let service = service::Service::new();
    let session_context = session::SessionContext(HashMap::new());
    router.get("/wx", wx::wx);
    router.post("/wx", wx::wx);
    router.get("/test",wx::test);

    router.post("/registerOwner",wx::register_owner);
    router.post("/registerPassenger",wx::register_passenger);
    router.post("/login",wx::login);
    router.post("/publish_trip",wx::publish_trip);
    router.post("/get_trips", wx::get_trips);

    router.get("/favicon.ico",wx::ico);
    
    let mut mount = Mount::new();
    mount.mount("/", router);
    mount.mount("/static/", Static::new(Path::new("./static/")));
    //mount.mount("/page/", Static::new(Path::new("./views/")));

    // middleware
    // ready to add middleware around mount entity
    let mut middleware = Chain::new(mount);

    middleware.link(PersistRead::<wx::WxInstance>::both(instance));
    middleware.link(PersistRead::<service::Service>::both(service.clone()));
    middleware.link(PersistState::<session::SessionContext>::both(session_context));
    middleware.link_before(PersistRead::<bodyparser::MaxBodyLength>::one(MAX_BODY_LENGTH));
    middleware.link_after(session::CheckSession);

    env_logger::init().unwrap();
    let mut hbse = HandlebarsEngine::new2();
    hbse.add(Box::new(DirectorySource::new("./templates/", ".hbs")));
    if let Err(r) = hbse.reload() {
        panic!("{}", r.description());
    }
    middleware.link_after(hbse);
     
    let ip = Ipv4Addr::new(0, 0, 0, 0);
    let port = get_server_port();
    info!("listen to {}", port);

    //db::test();
    
    thread::spawn(move || {
        //let service = service.clone();
        loop {
            thread::sleep_ms(60000);
            service.update_status();
        }
    });
    

    Iron::new(middleware).http(SocketAddrV4::new(ip, port)).unwrap();

}
/*
fn test() {
    let p = model::Passenger::new("2342523".to_string());
    let b = model::en_bson(p).unwrap();
    info!("{:?}",b);
    info!("{:?}", model::de_bson::<model::Passenger>(b).unwrap());
}
*/
