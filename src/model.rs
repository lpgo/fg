
use serde_xml::from_str;
use serde_xml::Error;
use std::option::Option;
use serde::{de,Deserialize, Serialize, Deserializer};
use bson::{Bson, Encoder, Decoder, DecoderError,Document};
use serde_json;
use std::collections::BTreeMap;
use std::fmt;
use chrono;
use iron::typemap::Key;
use rustc_serialize::json::{ToJson,Json};

include!(concat!(env!("OUT_DIR"), "/model.rs"));

