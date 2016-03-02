#![allow(dead_code)]
#[macro_use(bson, doc)]
extern crate bson;
extern crate mongodb;
extern crate quickersort;
extern crate sha1;
extern crate iron;
extern crate urlencoded;
extern crate persistent;
extern crate serde;
extern crate serde_xml;
extern crate hyper;
extern crate openssl;
extern crate bodyparser;
extern crate rustc_serialize;
extern crate uuid;
extern crate cookie;
extern crate serde_json;
extern crate handlebars_iron as hbs;
#[macro_use]
extern crate log;
extern crate chrono;

pub mod wx;
pub mod db;
pub mod model;
pub mod pay;
pub mod service;
pub mod session;
