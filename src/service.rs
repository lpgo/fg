use model::{Owner,Passenger,Trip,Line,PayResult,Order,OrderStatus,TripStatus};
use db::{Dao,ToDoc};
use iron::typemap::Key;
use chrono::{UTC,Local};
use serde_json;
use pay::{self,PrePay};
use config::ConfigManager;
use std::{io,fmt,error};
use hyper;
use persist;
use urlencoded;
use serde_xml;
use std::result;
use std::collections::{BTreeMap,HashSet,HashMap};
use serde::{de,Deserialize, Serialize, Deserializer};
use bson::{Bson, Encoder,EncoderError, Decoder, DecoderError,Document,oid};
use mongodb;
use uuid::Uuid;
use jsonway;
use rustc_serialize::json;
use std::sync::RwLock;
use std::sync::Arc;
use iron::{Request, Response, BeforeMiddleware, AfterMiddleware, IronResult};
use plugin::Plugin;

pub struct Service {
    db: Dao,
    cache: Arc<RwLock<Cache>>
}
struct Cache {
    trips: HashMap<String,Trip>,
    orders: HashMap<String,Order>,
    busy: HashSet<String>
}

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
    Other(String),
    DontHaveEnoughSeats,
    NoCache(String),
    UserBusy(String)
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
        	ServiceError::DontHaveEnoughSeats => write!(f,"this trip have not enough seats!"),
        	ServiceError::Other(ref s) => write!(f, "{}",s),
            ServiceError::NoCache(ref s) => write!(f,"{} no cache",s),
            ServiceError::UserBusy(ref s) => write!(f, "{} is busy",s)
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
		Service{db:Dao::new(),cache:Arc::new(RwLock::new(Cache::new()))}
	}

	pub fn add_owner(&self,o:Owner) -> Result<()> {
		self.db.add::<Owner>(o);
        Ok(())
	}
	pub fn add_passenger(&self,o:Passenger) -> Result<()> {
		self.db.add::<Passenger>(o);
        Ok(())
	}

	pub fn add_trip(&self,o:Trip) -> Result<()> {
		//o.start_time = o.start_time.with_timezone(&UTC);
        let mut o1 = o.clone();
		if let Ok(Some(Bson::ObjectId(oid))) = self.db.add::<Trip>(o) {
            o1._id = Some(Bson::ObjectId(oid));
            let mut c = self.cache.write().unwrap();
            c.add_busy(o1.openid.clone());
            c.add_trip(o1);
            Ok(())
        } else {
            Err(ServiceError::Other("can't cache trip,there is not oid".to_string()))
        }
	}

	pub fn get_user_by_id(&self,openid:&str) -> (Option<Owner>,Option<Passenger>) {
		let o = self.db.get_by_openid::<Owner>(openid).ok();
		let p = self.db.get_by_openid::<Passenger>(openid).ok();
		//warn!("openid is {}--{:?},{:?}",openid,o,p);
		(o,p)
	}

	pub fn get_new_trips(&self) -> Vec<Trip> {
        let c = self.cache.read().unwrap();
        let data = c.trips.values().filter(|t|t.status == "Prepare").map(|t|t.clone()).collect::<Vec<Trip>>();
        if data.is_empty() {
		    self.db.get_trip_by_status("Prepare")
        } else {
            data
        }
	}

	pub fn get_lines(&self) -> String {
		let data = self.db.get_all_lines();
		serde_json::to_string(&data).unwrap()
	}

	pub fn get_line_by_id(&self,id:u32) -> Result<Line> {
		self.db.get_line_by_id(id)
	}


	pub fn get_hot_lines(&self) -> String {
		let data = self.db.get_hot_lines();
		serde_json::to_string(&data).unwrap()
	}
    /*
	pub fn get_trip_by_oid(&self,oid:&str) -> Result<Trip> {
        let c = self.cache.read().unwrap();
        c.trips.get(oid).map(|trip|trip.clone()).ok_or(ServiceError::Other("can't fnid trip in cache".to_string())).or_else(|err|{
		    self.db.get_by_id::<Trip>(oid)
        })
	}
    */
    
	pub fn get_trip_info(&self,openid:&str) -> Result<String> {
		self.get_trip_by_openid(openid).and_then(|trip|{

			let oid = match trip._id.clone().unwrap() {
				Bson::ObjectId(id) => {
					id
				},
				_ => oid::ObjectId::new().unwrap()
			};
			let trip_id = format!("{}",oid);

			let mut orders = self.get_orders_by_trip_id(&trip_id);

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
                j.set("success",true);
			});
			serde_json::to_string(&rep.unwrap()).map_err(|err|ServiceError::SerdeJsonError(err))
		}).or_else(|_|{
            self.get_order_by_openid(openid).and_then(|order|{
                self.get_trip_by_id(&order.trip_id).and_then(|trip|{
			        let rep = jsonway::object(|j|{
				         j.set("trip",trip);
                         j.set("order",order);
                        j.set("success",true);
			        });
			        serde_json::to_string(&rep.unwrap()).map_err(|err|ServiceError::SerdeJsonError(err))
                })
            })
        })
	}

	pub fn get_passenger_trips(&self,openid:&str) -> Result<String> {
		self.get_order_by_openid(openid).and_then(|order|{
			self.get_trip_by_id(&order.trip_id)
		}).and_then(|trip|{
			serde_json::to_string(&trip).map_err(|err|ServiceError::SerdeJsonError(err))
		})
	}

	pub fn apply_trip(&self,oid:&str,openid:&str,count:&str) -> Result<String> {
		let order_id = Uuid::new_v4().simple().to_string();
		let msg = "pinchefei".to_string();
        if self.is_busy(openid){
            Err(ServiceError::UserBusy(openid.to_string()))
        } else {
	        self.get_trip_by_id(oid).and_then(|trip|{
                count.parse::<u32>().map_err(|err|ServiceError::Other(err.to_string())).and_then(|c|{
                    if trip.current_seat < c {
                        Err(ServiceError::DontHaveEnoughSeats)
                    } else {
		    	        self.get_line_by_id(trip.line_id).map(|line|line.price)
                    }
                })
		    }).and_then(|price|{
			    let prepay = PrePay::new(order_id, oid.to_owned(), msg, openid.to_owned(),price);
			    pay::pre_pay(prepay).map_err(|err|ServiceError::Other(err.to_string()))
		    }).map(|result|result.prepay_id)	
        }
	}

	pub fn buy_seats(&self,oid:&str,count:u32) -> bool {
		match self.get_trip_by_id(oid).map(|trip|{
			if trip.current_seat >= count {
				self.set_current_seats(oid,trip.current_seat - count);
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
		if let Err(err) = self.get_trip_by_id(&pay.attach).and_then(|trip|{
			self.get_line_by_id(trip.line_id)
		}).and_then(|line|{
			let mut order = Order::from_pay_result(pay,line.price,pay.total_fee/line.price);
			if !self.buy_seats(&pay.attach, order.count) {
				order.set_status(OrderStatus::PayFail);
			} else {
                self.set_busy(&pay.openid);
                self.get_trip_by_id(&pay.attach).and_then(|trip|{
                    if trip.current_seat <= 0 {
                        self.update_status(&pay.attach,TripStatus::Full);
                    }
                    Ok(())
                });
            }
			self.add_order(order)
		}) {
			warn!("pay success error : {}",err);
		}
	}


    pub fn submit_order(&self,openid:&str)  {
        {
            let mut c = self.cache.write().unwrap();
            c.orders.get_mut(openid).map(|o|o.set_status(OrderStatus::Submit));
        }
        self.db.update_order(openid,OrderStatus::Submit);
        self.not_busy(openid);
        let (finish,id) =  self.check_trip_finish(openid);
        if finish {
            self.do_finish(&id.unwrap());
        }

    } 

    pub fn update_trips_running(&self) {
        let mut c =self.cache.write().unwrap();
        let now = Local::now().timestamp();
        for (id,trip) in c.trips.iter_mut() {
            if now > trip.start_time {
                trip.set_status(TripStatus::Running);
                self.db.update_status(id,TripStatus::Running);
            }
        }
    }
	pub fn update_status(&self,id:&str,status:TripStatus) {
        let mut c = self.cache.write().unwrap();
        c.trips.get_mut(id).ok_or(ServiceError::NoCache(id.to_string())).map(|trip|{
           trip.set_status(status.clone());
        });
        self.db.update_status(id,status);
    }

    fn do_finish(&self,id:&str) {
            self.update_status(&id,TripStatus::Finish);
            self.get_trip_by_id(&id).map(|trip|trip.openid).and_then(|owner_id|{
                self.not_busy(&owner_id);
                let fee = self.get_all_fee(&id);
                if fee <= 0 {
                    Err(ServiceError::Other("can't get all fee".to_string()))
                } else {
                    let final_fee = fee as f32 * 0.95;
                    pay::pay_to_client(&owner_id,format!("{:0}",final_fee).as_ref());
                    Ok(())
                }
            });
            self.move_to_history(id);
    }

    fn move_to_history(&self,id:&str) {
        self.get_trip_by_id(id).and_then(|trip|{
            self.delete_trip_by_id(id);
            self.db.add_history::<Trip>(trip);
            Ok(())
        }).and_then(|_|{
            let orders = self.get_orders_by_trip_id(id);
            {
                let openids = orders.iter().map(|order|order.openid.as_ref()).collect();
                self.delete_orders_by_openids(openids);
            }
            self.db.add_orders_history(orders);
            Ok(())
        });
    }

    fn check_trip_finish(&self,openid:&str) -> (bool,Option<String>) {
        match self.get_order_by_openid(openid).map(|order|order.trip_id).map(|trip_id|{
            let mut all_submit = true;
            for o in self.get_orders_by_trip_id(&trip_id) {
               match o.get_status() {
                   OrderStatus::PaySuccess | OrderStatus::Request => {
                       all_submit = false;
                       break;
                   },
                   OrderStatus::PayFail => {},
                   OrderStatus::Submit => {},
                   OrderStatus::Refund => {}
               }
            }
            if all_submit {
                (all_submit,Some(trip_id))
            } else {
                (all_submit,None)
            }
        }) {
            Ok(result) => result,
            Err(err) => {
                warn!("check trip finish error : {}",err);
                (false,None)
            }
        }
    }
    fn set_busy(&self,openid:&str) {
        let mut c = self.cache.write().unwrap();
        c.busy.insert(openid.to_string());
    }
    pub fn is_busy(&self,openid:&str) -> bool {
        let c = self.cache.read().unwrap();
        c.busy.contains(openid)
    }
    fn not_busy(&self,openid:&str) {
        let mut c = self.cache.write().unwrap();
        c.busy.remove(openid);
    }
    fn get_trip_by_openid(&self,openid:&str) -> Result<Trip> {
        let c = self.cache.read().unwrap();
        for(_,value) in c.trips.iter() {
            if value.openid == openid {
                return Ok(value.clone());
            }
        }
        self.db.get_by_openid::<Trip>(openid)
    }
    fn get_orders_by_trip_id(&self,id:&str) -> Vec<Order> {
        let c = self.cache.read().unwrap();
        let data = c.orders.values().filter(|t|t.trip_id == id).map(|t|t.clone()).collect::<Vec<Order>>();
        if data.is_empty() {
            self.db.get_orders_by_trip_id(id)
        } else {
            data
        }
    }
    fn get_order_by_openid(&self,openid:&str) -> Result<Order> {
        let c = self.cache.read().unwrap();
        c.orders.get(openid).ok_or(ServiceError::NoCache(openid.to_string())).map(|o|o.clone()).or_else(|_|{
            self.db.get_by_openid::<Order>(openid)
        })
    }
    pub fn get_trip_by_id(&self,id:&str) -> Result<Trip> {
        let c = self.cache.read().unwrap();
        c.trips.get(id).map(|o|o.clone()).ok_or(ServiceError::NoCache(id.to_string())).or_else(|_|{
            self.db.get_by_id::<Trip>(id)
        })
    }
    fn set_current_seats(&self,id:&str,count:u32) {
        let mut c = self.cache.write().unwrap();
        c.trips.get_mut(id).map(|trip|trip.current_seat = count);
        self.db.set_current_seats(id,count);
    }
    fn add_order(&self,o:Order) -> Result<Option<Bson>> {
        let mut c = self.cache.write().unwrap();
        c.orders.insert(o.openid.clone(),o.clone());
        self.db.add::<Order>(o)
    }
    fn delete_trip_by_id(&self,id:&str) {
        let mut c = self.cache.write().unwrap();
        c.trips.remove(id);
        self.db.delete::<Trip>(id);
    }
    fn delete_orders_by_openids(&self,openids:Vec<&str>) {
        let mut c = self.cache.write().unwrap();
        for openid in &openids {
            c.orders.remove(*openid);
        }
        self.db.delete_many_orders(openids);
    }
    fn get_all_fee(&self,id:&str) -> u32 {
        match self.get_trip_by_id(id).map(|trip|trip.price * (trip.seat_count-trip.current_seat)) {
            Ok(fee) => fee,
            Err(err) => {
                warn!("get total fee error : {}",err);
                0
            }
        }
    }
}

impl Cache {
    pub fn new() -> Cache{
        Cache{trips:HashMap::new(),busy:HashSet::new(),orders:HashMap::new()}
    }
    pub fn add_trip(&mut self,t:Trip) {
        if let Some(Bson::ObjectId(oid)) = t._id.clone() {
            let id = format!("{}",oid);
            self.trips.insert(id,t);
        } else {
            warn!("this trip has not a oid!");
        }
    }
    pub fn add_busy(&mut self,openid:String) {
        self.busy.insert(openid);
    }
}

impl Key for Service {
    type Value = Service;
}


impl Clone for Service {
	fn clone(&self) -> Service {
		Service{db:self.db.clone(),cache:self.cache.clone()}
	}
}

impl<'a, 'b> Plugin<Request<'a, 'b>> for Service {
    type Error = ServiceError;
    fn eval(req: &mut Request<'a, 'b>) -> Result<Service> {
        req.extensions.get::<Service>().cloned().ok_or(ServiceError::Other("can't find Service".to_string()))
    }
}


impl BeforeMiddleware for Service {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        req.extensions.insert::<Service>(self.clone());
        Ok(())
    }
}


impl AfterMiddleware for Service {
    fn after(&self, _: &mut Request, mut res: Response) -> IronResult<Response> {
        res.extensions.insert::<Service>(self.clone());
        Ok(res)
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
	strs.insert("mch_id",&mchid);
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

