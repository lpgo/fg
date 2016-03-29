
use serde_xml::from_str;
use serde_xml::Error;
use std::option::Option;
use serde::{de,Deserialize, Serialize, Deserializer};
use bson::{Bson, Encoder, Decoder, DecoderError,Document};
use serde_json::value::{self, Value};
use std::collections::BTreeMap;
use chrono;
use iron::typemap::Key;

include!(concat!(env!("OUT_DIR"), "/model.rs"));

