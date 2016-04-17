use model::{Owner,Passenger,Trip,Line};
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
use serde::{de,Deserialize, Serialize, Deserializer};
use bson::{Bson, Encoder,EncoderError, Decoder, DecoderError,Document,oid};
use mongodb;

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

	pub fn get_user_by_id(&self,openid:&String) -> (Option<Owner>,Option<Passenger>) {
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

	//todo
	pub fn apply_trip(&self,oid:&str,openid:&str,ip:String) -> Result<String> {
		let appid = ConfigManager::get_config_str("app", "appid");
		let mch_id = ConfigManager::get_config_str("app", "mchid");
		let msg = "pinchefei".to_string();
		let prepay = PrePay::new(appid, mch_id, oid.to_owned(), msg, ip, openid.to_owned());
		if let Ok(result) = pay::pre_pay(prepay) {
			Ok(result.prepay_id.clone())
		} else {
			Err(ServiceError::Other("applay trip error".to_string()))
		}
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

