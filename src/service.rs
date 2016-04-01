use model::{Owner,Passenger,Trip};
use db::Dao;
use bson::Document;
use iron::typemap::Key;
use chrono::UTC;
use serde_json;
use pay::{self,PrePay};
use config::ConfigManager;

pub struct Service(Dao);



impl Service {

	pub fn new() -> Service {
		Service(Dao::new())
	}

	pub fn add_owner(&self,o:Owner) -> Result<(),()> {
		self.0.add::<Owner>(o).unwrap();
		Result::Ok(())
	}
	pub fn add_passenger(&self,o:Passenger) -> Result<(),()> {
		self.0.add::<Passenger>(o).unwrap();
		Result::Ok(())
	}

	pub fn add_trip(&self,o:Trip) -> Result<(),()> {
		//o.start_time = o.start_time.with_timezone(&UTC);
		self.0.add::<Trip>(o).unwrap();
		Result::Ok(())
	}

	pub fn get_user_by_id(&self,openid:&String) -> (Option<Owner>,Option<Passenger>) {
		let o = self.0.get_by_openid::<Owner>(openid).ok();
		let p = self.0.get_by_openid::<Passenger>(openid).ok();
		(o,p)
	}

	pub fn get_new_trips(&self) -> String {
		let data = self.0.get_trip_by_status("Prepare");
		info!("{:?}",data);
		serde_json::to_string(&data).unwrap()
	}

	pub fn get_lines(&self) -> String {
		let data = self.0.get_all_lines();
		serde_json::to_string(&data).unwrap()
	}

	pub fn get_hot_lines(&self) -> String {
		let data = self.0.get_hot_lines();
		serde_json::to_string(&data).unwrap()
	}



	//todo
	pub fn apply_trip(&self,oid:&str,openid:&str,ip:String) -> Result<String,()> {
		let appid = ConfigManager::get_config_str("app", "appid");
		let mch_id = ConfigManager::get_config_str("app", "mchid");
		let msg = "pinchefei".to_string();
		let prepay = PrePay::new(appid, mch_id, oid.to_owned(), msg, ip, openid.to_owned());
		if let Ok(result) = pay::pre_pay(prepay) {
			Ok(result.prepay_id.clone())
		} else {
			Err(())
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

