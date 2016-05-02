use model::{Owner,Passenger,Trip,Line,PayResult,Order,OrderStatus};
use db::{Dao,ToDoc};
use iron::typemap::Key;
use chrono::UTC;
use serde_json;
use pay::{self,PrePay};
use config::ConfigManager;
use std::{io,fmt,error};
use hyper;
use persist;
use urlencoded;
use serde_xml;
use std::result;
use std::collections::BTreeMap;
use serde::{de,Deserialize, Serialize, Deserializer};
use bson::{Bson, Encoder,EncoderError, Decoder, DecoderError,Document,oid};
use mongodb;
use uuid::Uuid;
use jsonway;
use rustc_serialize::json;

pub struct Service(Dao);

 #[derive(Debug)]
pub enum ServiceError{
    IoError(io::Error),
    HyperError(hyper::Error),
    ParameterError(String),
    SerdeJsonError(serde_json::Error),
    PersistentError(persist::PersistentError),
    UrlDecodingError(urlencoded::UrlDecodingError),
    NoLogin,
    SerdeXmlError(serde_xml::Error),
    BsonEncoderError(EncoderError),
    CanNotSerializeToDoc(String),
    BsonDecoderError(DecoderError),
    MongodbError(mongodb::Error),
    BsonOidError(oid::Error),
    JsonEncoderError(json::EncoderError),
    Other(String)

}

pub type Result<T> = result::Result<T, ServiceError>;

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
        	ServiceError::IoError(ref e) => e.fmt(f),
        	ServiceError::HyperError(ref e) => e.fmt(f),
        	ServiceError::ParameterError(ref s) => write!(f,"{} can not find!",s),
        	ServiceError::CanNotSerializeToDoc(ref s) => write!(f,"{} can not serialize to a document,it may other bson type!",s),
        	ServiceError::SerdeJsonError(ref e) => e.fmt(f),
        	ServiceError::PersistentError(ref e) => e.fmt(f),
        	ServiceError::UrlDecodingError(ref e) => e.fmt(f),
        	ServiceError::SerdeXmlError(ref e) => e.fmt(f),
        	ServiceError::BsonEncoderError(ref e) => e.fmt(f),
        	ServiceError::BsonDecoderError(ref e) => e.fmt(f),
        	ServiceError::BsonOidError(ref e) => e.fmt(f),
        	ServiceError::MongodbError(ref e) => e.fmt(f),
        	ServiceError::JsonEncoderError(ref e) => e.fmt(f),
        	ServiceError::NoLogin => write!(f,"you are not  login!"),
        	ServiceError::Other(ref s) => write!(f, "{}",s)
         }
     }
}

impl error::Error for ServiceError {
	fn description(&self) -> &str {
		"all error in service"
	}
	fn cause(&self) -> Option<&error::Error> {
		None
	}
}



impl Service {

	pub fn new() -> Service {
		Service(Dao::new())
	}

	pub fn add_owner(&self,o:Owner) -> Result<()> {
		self.0.add::<Owner>(o)
	}
	pub fn add_passenger(&self,o:Passenger) -> Result<()> {
		self.0.add::<Passenger>(o)
	}

	pub fn add_trip(&self,o:Trip) -> Result<()> {
		//o.start_time = o.start_time.with_timezone(&UTC);
		self.0.add::<Trip>(o)
	}

	pub fn get_user_by_id(&self,openid:&str) -> (Option<Owner>,Option<Passenger>) {
		let o = self.0.get_by_openid::<Owner>(openid).ok();
		let p = self.0.get_by_openid::<Passenger>(openid).ok();
		//warn!("openid is {}--{:?},{:?}",openid,o,p);
		(o,p)
	}

	pub fn get_new_trips(&self) -> Vec<Trip> {
		self.0.get_trip_by_status("Prepare")
	}

	pub fn get_lines(&self) -> String {
		let data = self.0.get_all_lines();
		serde_json::to_string(&data).unwrap()
	}

	pub fn get_line_by_id(&self,id:u32) -> Result<Line> {
		self.0.get_line_by_id(id)
	}

	pub fn get_hot_lines(&self) -> String {
		let data = self.0.get_hot_lines();
		serde_json::to_string(&data).unwrap()
	}

	pub fn get_trip_by_oid(&self,oid:&str) -> Result<Trip> {
		self.0.get_by_id::<Trip>(oid)
	}

	pub fn get_owner_trips(&self,openid:&str) -> Result<String> {
		self.0.get_by_openid::<Trip>(openid).and_then(|trip|{

			let oid = match trip._id.clone().unwrap() {
				Bson::ObjectId(id) => {
					id
				},
				_ => oid::ObjectId::new().unwrap()
			};
			let trip_id = format!("{}",oid);

			let mut orders = self.0.get_orders_by_trip_id(&trip_id);

			orders = orders.into_iter().map(|mut order|{
				match self.get_user_by_id(&order.openid) {
					(Some(o),Some(p)) => {
						order.tel = Some(o.tel);
					},
					(Some(o),None) => {
						order.tel = Some(o.tel);
					},
					(None,Some(p)) => {
						order.tel = Some(p.tel);
					},
					(None,None) => {},
				}
				order
			}).collect();

			let rep = jsonway::object(|j|{
				j.set("trip",trip);
				j.set("orders",orders);
			});
			json::encode(&rep.unwrap()).map_err(|err|ServiceError::JsonEncoderError(err))
		})
	}

	//todo
	pub fn apply_trip(&self,oid:&str,openid:&str) -> Result<String> {
		let order_id = Uuid::new_v4().to_simple_string();
		let msg = "pinchefei".to_string();
		self.get_trip_by_oid(oid).and_then(|trip|{
			self.get_line_by_id(trip.line_id).map(|line|line.price)
		}).and_then(|price|{
			let prepay = PrePay::new(order_id, oid.to_owned(), msg, openid.to_owned(),price);
			pay::pre_pay(prepay).map_err(|err|ServiceError::Other(err.to_string()))
		}).map(|result|result.prepay_id)
	}

	pub fn buy_seats(&self,oid:&str,count:u32) -> bool {
		match self.get_trip_by_oid(oid).map(|trip|{
			if trip.current_seat >= count {
				self.0.set_current_seats(oid,trip.current_seat - count);
				true
			} else {
				false
			}
		}) {
			Ok(b) => b,
			Err(err) => {
				warn!("buy seats error : {}",err);
				false
			}
		}
	}

	pub fn pay_success(&self,pay:&PayResult) {
		if let Err(err) = self.get_trip_by_oid(&pay.attach).and_then(|trip|{
			self.get_line_by_id(trip.line_id)
		}).and_then(|line|{
			let mut order = Order::from_pay_result(pay,line.price,pay.total_fee/line.price);
			if !self.buy_seats(&pay.attach, order.count) {
				order.set_status(OrderStatus::PayFail);
			}
			self.0.add::<Order>(order)
		}) {
			warn!("pay success error : {}",err);
		}
	}

	pub fn submit_order(&self,order_id:&str) -> Result<()> {
		self.0.update_order(order_id,OrderStatus::Submit)
	}

	pub fn update_status(&self) {
		if let Ok(_) = self.0.update_status() {
			warn!("update_status success !")
		} else {
			warn!("update_status error!")
		}
	}
}

impl Key for Service {
    type Value = Service;
}


impl Clone for Service {
	fn clone(&self) -> Service {
		Service(self.0.clone())
	}
}

pub fn de_xml<T>(data:&str) -> Result<T> where T:Deserialize{
	serde_xml::from_str(data).map_err(|err|ServiceError::SerdeXmlError(err))
}

pub fn en_bson<T>(data:T) -> Result<Document> where T:Serialize+ToDoc{
    let mut e = Encoder::new();
    data.serialize(&mut e).map_err(|err|ServiceError::BsonEncoderError(err)).and_then(|_|{
    	e.bson().map_err(|err|ServiceError::BsonEncoderError(err)).and_then(|b|{
    		 if let Bson::Document(d) = b {
	                    Ok(d)
	                } else {
	                    Err(ServiceError::CanNotSerializeToDoc(T::get_name().to_string()))
	                }
    	})
    })
}

pub fn de_bson<T>(data:Document) -> Result<T> where T:Deserialize {
    let mut d = Decoder::new(Bson::Document(data));
    Deserialize::deserialize(&mut d).map_err(|err|ServiceError::BsonDecoderError(err))
}

pub fn check_pay_result(result:&PayResult) -> bool {
	let api_key = ConfigManager::get_config_str("app", "apikey");
	let appid = ConfigManager::get_config_str("app", "appid");
	let mchid = ConfigManager::get_config_str("app", "mchid");
	let cash_fee = format!("{}",result.cash_fee);
	let total_fee = format!("{}",result.total_fee);
	let mut strs:BTreeMap<&str,&str> = BTreeMap::new();
	strs.insert("appid",&appid);
	strs.insert("mchid",&mchid);
	strs.insert("nonce_str",&result.nonce_str);
	strs.insert("is_subscribe",&result.is_subscribe);
	strs.insert("openid",&result.openid);
	strs.insert("fee_type",&result.fee_type);
	strs.insert("cash_fee",&cash_fee);
	strs.insert("bank_type",&result.bank_type);
	strs.insert("attach",&result.attach);
	strs.insert("out_trade_no",&result.out_trade_no);
	strs.insert("result_code",&result.result_code);
	strs.insert("total_fee",&total_fee);
	strs.insert("return_code",&result.return_code);
	strs.insert("time_end",&result.time_end);
	strs.insert("trade_type",&result.trade_type);
	strs.insert("transaction_id",&result.transaction_id);
	let mut ss = String::new();
	for (k,v) in strs {
		ss.push_str(k);
		ss.push('=');
		ss.push_str(v);
		ss.push('&');
	}
	ss.push_str("key=");
	ss.push_str(&api_key);
	let sign = pay::to_md5(&ss);
	sign == result.sign
}

