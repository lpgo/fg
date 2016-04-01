use mongodb::Client;
use bson::{Document,Bson};
use std::sync::Arc;
use mongodb::ThreadedClient;
use mongodb::db::{ThreadedDatabase,DatabaseInner};
use iron::typemap::Key;
use model;
use serde::{Deserialize, Serialize, Deserializer};
use rustc_serialize::json;
use chrono::offset::local::Local;
use config::ConfigManager;


pub trait ToDoc {
	fn get_name() -> &'static str;
}

impl ToDoc for model::Passenger {
	fn get_name() -> &'static str {
		"Passenger"
	}
}

impl ToDoc for model::Owner {
	fn get_name() -> &'static str {
		"Owner"
	}
}

impl ToDoc for model::Seat {
	fn get_name() -> &'static str {
		"Seat"
	}
}

impl ToDoc for model::Trip {
	fn get_name() -> &'static str {
		"Trip"
	}
}

impl ToDoc for model::Line {
    fn get_name() -> &'static str {
        "Line"
    }
}

pub struct Dao(Arc<DatabaseInner>);

impl Dao {

    pub fn new() -> Dao {
        Dao(get_db())
    }

    pub fn add<T>(&self,t:T) -> Result<(),String> where T:ToDoc+Serialize{
        let coll = self.0.collection(T::get_name());
        match coll.insert_one(model::en_bson(t).unwrap(),None) {
        	Ok(_) => Ok(()),
        	Err(_) => Err(format!("insert one {} error!",T::get_name()))
        }		
    }

    pub fn get_by_openid<T>(&self,open_id:&String) -> Result<T,()> where T:ToDoc+Deserialize{
        let coll = self.0.collection(T::get_name());
        let mut doc = Document::new();
        doc.insert("open_id",open_id.clone());
        if let Ok(Some(user)) = coll.find_one(Some(doc),None) {
        	model::de_bson::<T>(user)
        } else {
        	Err(())
        }	
    }

    pub fn get_by_id<T>(&self,id:Bson) -> Result<T,()>  where T:ToDoc+Deserialize {
        let coll = self.0.collection(T::get_name());
        let mut doc = Document::new();
        //let Bson::ObjectId(_id) = id;
        doc.insert("_id",id);
        if let Ok(Some(user)) = coll.find_one(Some(doc),None) {
            model::de_bson::<T>(user)
        } else {
            Err(())
        }   
    }

    
    pub fn get_trip_by_status(&self,status:&str) -> Vec<model::Trip>{
        let coll = self.0.collection(model::Trip::get_name());
        let mut doc = Document::new();
        //let Bson::ObjectId(_id) = id;
        doc.insert("status",status);
        let mut data:Vec<model::Trip> = Vec::new();
        if let Ok(c) = coll.find(Some(doc),None) {
            for result in c {
                let value = result.unwrap();
                data.push(model::de_bson::<model::Trip>(value).unwrap());
            }
        } 
        data
    }

    pub fn get_all_lines(&self) -> Vec<model::Line> {
        let coll = self.0.collection(model::Line::get_name());
        coll.find(None,None).map(|cursor|{
            cursor.map(|result| {
                let value = result.unwrap();
                model::de_bson::<model::Line>(value).unwrap()
            }).collect()
        }).unwrap()
    }

    pub fn get_hot_lines(&self) -> Vec<model::Line> {
        let coll = self.0.collection(model::Line::get_name());
         let mut doc = Document::new();
        doc.insert("hot",true);
        coll.find(Some(doc),None).map(|cursor|{
            cursor.map(|result| {
                let value = result.unwrap();
                model::de_bson::<model::Line>(value).unwrap()
            }).collect()
        }).unwrap()
    }
   
    //db.Trip.update({"owner_id":"openid"},{"$set":{"status":"Finish"}})
    pub fn update_status(&self) -> Result<(),()> {
        let coll = self.0.collection("Trip");
        let now = Local::now().timestamp();
        warn!("now is {}",now);
        let mut doc = Document::new();
        let mut update_doc = Document::new();
        update_doc.insert("$set",doc!{"status" => "Running"});
        doc.insert("start_time",doc!{"$lte" => now});
        if let Ok(_) = coll.update_many(doc,update_doc,None) {
            return Ok(());
        }
        return Err(());
    }
    
}

impl Key for Dao {
    type Value = Dao;
}

impl Clone for Dao {
   fn clone(&self) -> Dao {
        Dao(self.0.clone())
    }
}

fn get_db() -> Arc<DatabaseInner> {
	let client = Client::connect("localhost", 27017)
        .ok().expect("Failed to initialize standalone client.");
    let db_name = ConfigManager::get_config_str("app", "dbname");
    let db_user = ConfigManager::get_config_str("app", "dbuser");
    let db_pwd = ConfigManager::get_config_str("app", "dbpwd");
    let db = client.db(&db_name);
    db.auth(&db_user,&db_pwd).unwrap();
    db
}
