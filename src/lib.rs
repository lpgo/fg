#![allow(dead_code)]
#[macro_use(bson, doc)]
extern crate bson;
extern crate mongodb;
extern crate quickersort;
extern crate sha1;
extern crate iron;
extern crate urlencoded;
extern crate serde;
extern crate serde_xml;
extern crate hyper;
extern crate openssl;
extern crate rustc_serialize;
extern crate uuid;
extern crate cookie;
extern crate serde_json;
extern crate handlebars_iron as hbs;
extern crate plugin;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate chrono;
extern crate jsonway;
extern crate md5;
extern crate toml;
extern crate url;
extern crate rand;
#[macro_use] extern crate lazy_static;

macro_rules! res_template {
    ($name:expr, $data:expr,$resp:expr) => ({
	$resp.set_mut(Template::new($name, $data)).set_mut(status::Ok);
	Ok($resp)
    }) 
}

macro_rules! redirect {
    ($url:expr) => ({
    	let domain = ConfigManager::get_config_str("app", "domain");
           	let urlstr = domain+$url;
	let mut response = Response::new();
	let url = Url::parse(&urlstr).unwrap();
	response.set_mut(status::Found).set_mut(Redirect(url));
	Ok(response)
    })  
}

macro_rules! redirect2 {
    ($url:expr,$resp:expr) => ({
    let domain = ConfigManager::get_config_str("app", "domain");
    let urlstr = domain+$url;
	let url = Url::parse(&urlstr).unwrap();
	$resp.set_mut(status::Found).set_mut(Redirect(url));
	Ok($resp)
    })  
}

macro_rules! get_db {
    ($c:expr) => ({
        let client = $c.lock().unwrap(); 
        let db_name = ConfigManager::get_config_str("app", "dbname");
        let db_user = ConfigManager::get_config_str("app", "dbuser");
        let db_pwd = ConfigManager::get_config_str("app", "dbpwd");
        let db = client.db(&db_name);
        db.auth(&db_user,&db_pwd).unwrap();
        warn!("auth end");
        db
    })
}

pub mod wx;
pub mod db;
pub mod model;
pub mod pay;
pub mod service;
pub mod session;
pub mod config;
pub mod persist;


