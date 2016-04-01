#![allow(dead_code)]
extern crate iron;
extern crate weixin;
extern crate mount;
extern crate staticfile;
extern crate router;
extern crate mongodb;
extern crate handlebars_iron as hbs;
#[macro_use] extern crate log;
extern crate env_logger;


use iron::prelude::*;
use weixin::wx;
use weixin::service;
use weixin::session;
use weixin::pay::{PrePay,pre_pay,pay_to_client};

use weixin::model;


use mount::Mount;
use staticfile::Static;
use router::Router;
use weixin::persist::Read as PersistRead;
use weixin::persist::State as PersistState;
use hbs::{HandlebarsEngine, DirectorySource};

use std::path::Path;
use std::collections::HashMap;
use std::env;
use std::net::{SocketAddrV4,Ipv4Addr};
use std::error::Error;
use std::thread;
use std::time::Duration;

fn get_server_port() -> u16 {
    env::var("PORT").unwrap_or("80".to_string()).parse().unwrap()
}

fn main() {
    let mut instance = wx::WxInstance::new();
    let mut router = Router::new();
    let service = service::Service::new();
    let session_context = session::SessionContext(HashMap::new());
    router.get("/wx", wx::wx);
    router.post("/wx", wx::wx);
    router.get("/test",wx::test);

    router.post("/registerOwner",wx::register_owner);
    router.post("/registerPassenger",wx::register_passenger);
    //router.post("/login",wx::login);
    router.post("/publishTrip",wx::publish_trip);
    router.post("/getTrips", wx::get_trips);
    router.post("/applyTrip",wx::apply_trip);
    router.post("/getLines", wx::get_lines);
    router.post("/getHotLines",wx::get_hot_lines);

    //template
    router.get("/pinche/index.html",wx::index_template);
    router.get("/pinche/myInfo", wx::my_info_template);

    router.get("/favicon.ico",wx::ico);
    router.get("/",wx::index);
    
    let mut mount = Mount::new();
    mount.mount("/", router);
    mount.mount("/static/", Static::new(Path::new("./static/")));
    //mount.mount("/page/", Static::new(Path::new("./views/")));

    // middleware
    // ready to add middleware around mount entity
    let mut middleware = Chain::new(mount);

    let persist_instance= PersistState::<wx::WxInstance>::both(instance);
    let therad_instance = persist_instance.0.clone();
    middleware.link(persist_instance);
    middleware.link(PersistRead::<service::Service>::both(service.clone()));
    middleware.link(PersistState::<session::SessionContext>::both(session_context));
    //middleware.link_after(session::CheckSession);

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

   
    
    thread::spawn(move || {
        //let service = service.clone();
        loop {
            thread::sleep(Duration::from_secs(60));
            let mut inst = therad_instance.data.write().unwrap();
            if inst.access_token_expires <= 120 {
                inst.get_access_token();
            } else {
                inst.access_token_expires -=60;
            }
            service.update_status();
        }
    });

    Iron::new(middleware).http(SocketAddrV4::new(ip, port)).unwrap();

}


