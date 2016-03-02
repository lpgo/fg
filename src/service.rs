use model::{Owner,Passenger,Trip};
use db::Dao;
use bson::Document;
use iron::typemap::Key;
use chrono::UTC;
use serde_json;

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

	pub fn get_user_by_id(&self,open_id:&String) -> (Option<Owner>,Option<Passenger>) {
		let o = self.0.get_by_openid::<Owner>(open_id).ok();
		let p = self.0.get_by_openid::<Passenger>(open_id).ok();
		info!("{:?}", p);
		(o,p)
	}

	pub fn get_new_trips(&self) -> String {
		let data = self.0.get_trip_by_status("Prepare");
		info!("{:?}",data);
		serde_json::to_string(&data).unwrap()
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

