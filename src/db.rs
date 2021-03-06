use mongodb::Client;
use bson::{Document,Bson,oid};
use std::sync::Arc;
use std::sync::{Mutex,MutexGuard};
use mongodb::db::{ThreadedDatabase,Database};
use mongodb::ThreadedClient;
use iron::typemap::Key;
use model;
use service;
use serde::{Deserialize, Serialize, Deserializer};
use rustc_serialize::json;
use chrono::offset::local::Local;
use config::ConfigManager;
use std::result;

pub type Result<T> = result::Result<T, service::ServiceError>;

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

impl ToDoc for model::Order {
	fn get_name() -> &'static str {
		"Order"
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

pub struct Dao(Client);
impl Dao {

    pub fn new() -> Dao {
        Dao(get_db())
    }
    
    pub fn add<T>(&self,t:T) -> Result<Option<Bson>> where T:ToDoc+Serialize{
        let coll = self.get_db().collection(T::get_name());
       coll.insert_one(service::en_bson(t).unwrap(),None).map(|r|r.inserted_id).map_err(|err|service::ServiceError::MongodbError(err)) 
    }

    pub fn delete<T>(&self,id:&str) where T:ToDoc+Serialize {
        let coll = self.get_db().collection(T::get_name());
        oid::ObjectId::with_string(id).map_err(|err|service::ServiceError::BsonOidError(err)).map(|o|Bson::ObjectId(o)).and_then(|oid|{
            let mut doc = Document::new();
            doc.insert("_id",oid);
            coll.delete_one(doc,None);
            Ok(())
        });
    }

    pub fn delete_by_openid<T>(&self,id:&str) where T:ToDoc+Serialize {
        let coll = self.get_db().collection(T::get_name());
        let mut doc = Document::new();
        doc.insert("openid",id);
        coll.delete_many(doc,None);
    }
    
    pub fn delete_many_orders(&self,openids:Vec<&str>) {
        let coll = self.get_db().collection("Order");
        let mut doc = Document::new();
        let openid_bson = openids.iter().map(|openid|Bson::String(openid.to_string())).collect();
        doc.insert("$in",Bson::Array(openid_bson));
        let mut doc1 = Document::new();
        doc1.insert("openid",doc);
        coll.delete_many(doc1,None);
    }
    
    pub fn add_history<T>(&self,t:T) where T:ToDoc+Serialize {
        let coll_name:&str = &format!("{}_history",T::get_name());
        let coll = self.get_db().collection(coll_name);
        coll.insert_one(service::en_bson(t).unwrap(),None);
    }
    pub fn add_orders_history(&self,orders:Vec<model::Order>) {
        let coll = self.get_db().collection("Order_history");
        let docs = orders.iter().map(|order|{
            service::en_bson(order.clone()).unwrap()
        }).collect();
        coll.insert_many(docs,None);
    }

    pub fn get_by_openid<T>(&self,openid:&str) -> Result<T> where T:ToDoc+Deserialize{
        let coll = self.get_db().collection(T::get_name());
        let mut doc = Document::new();
        doc.insert("openid",openid.clone());
        coll.find_one(Some(doc),None).map_err(|err|service::ServiceError::MongodbError(err)).and_then(|op|{
            op.ok_or(service::ServiceError::Other("not find by this openid".to_string())).and_then(|doc|{
                service::de_bson::<T>(doc)
            })
        })
    }

    pub fn get_by_id<T>(&self,id:&str) -> Result<T>  where T:ToDoc+Deserialize {
        let coll = self.get_db().collection(T::get_name());
        let mut doc = Document::new();
        oid::ObjectId::with_string(id).map_err(|err|service::ServiceError::BsonOidError(err)).map(|o|Bson::ObjectId(o)).and_then(|oid|{
            doc.insert("_id",oid);
            coll.find_one(Some(doc),None).map_err(|err|service::ServiceError::MongodbError(err)).and_then(|op|{
                op.ok_or(service::ServiceError::Other("not find by this _id".to_string())).and_then(|doc|{
                    service::de_bson::<T>(doc)
                })
            })
        })
    }

    pub fn get_trip_by_status(&self,status:&str) -> Vec<model::Trip>{
        let coll = self.get_db().collection(model::Trip::get_name());
        let mut doc = Document::new();
        //let Bson::ObjectId(_id) = id;
        doc.insert("status",status);
        let mut data:Vec<model::Trip> = Vec::new();
        if let Ok(c) = coll.find(Some(doc),None) {
            for result in c {
                let value = result.unwrap();
                data.push(service::de_bson::<model::Trip>(value).unwrap());
            }
        } 
        data
    }

    pub fn get_all_lines(&self) -> Vec<model::Line> {
        let coll = self.get_db().collection(model::Line::get_name());
        coll.find(None,None).map(|cursor|{
            cursor.map(|result| {
                let value = result.unwrap();
                service::de_bson::<model::Line>(value).unwrap()
            }).collect()
        }).unwrap()
    }

    pub fn get_line_by_id(&self,id:u32) -> Result<model::Line> {
        let coll = self.get_db().collection(model::Line::get_name());
        let mut doc = Document::new();
        //let Bson::ObjectId(_id) = id;
        doc.insert("id",id);
        coll.find_one(Some(doc),None).map_err(|err|service::ServiceError::MongodbError(err)).and_then(|od|{
            od.ok_or(service::ServiceError::Other("not find by this id".to_string())).and_then(|doc|{
                service::de_bson::<model::Line>(doc)
            })
        })
    }

    pub fn get_hot_lines(&self) -> Vec<model::Line> {
        let coll = self.get_db().collection(model::Line::get_name());
         let mut doc = Document::new();
        doc.insert("hot",true);
        coll.find(Some(doc),None).map(|cursor|{
            cursor.map(|result| {
                let value = result.unwrap();
                service::de_bson::<model::Line>(value).unwrap()
            }).collect()
        }).unwrap()
    }

    pub fn update_order(&self,order_id:&str,status:model::OrderStatus) -> Result<()> {
        let coll = self.get_db().collection("Order");
        let mut doc = Document::new();
        let mut update_doc = Document::new();
        let st:&str = &format!("{}",status);
        update_doc.insert("$set",doc!{"status" => st});
        doc.insert("openid",order_id);
        coll.update_one(doc,update_doc,None).map(|_|()).map_err(|err|service::ServiceError::MongodbError(err))
    }

    pub fn set_current_seats(&self,id:&str,seats:u32) -> Result<()> {
        let coll = self.get_db().collection("Trip");
        let mut doc = Document::new();
        let mut update_doc = Document::new();
        update_doc.insert("$set",doc!{"current_seat" => seats});
        oid::ObjectId::with_string(id).map_err(|err|service::ServiceError::BsonOidError(err)).map(|o|Bson::ObjectId(o)).and_then(|oid|{
            doc.insert("_id",oid);
            coll.update_one(doc,update_doc,None).map(|_|()).map_err(|err|service::ServiceError::MongodbError(err))
        })
    }

    pub fn get_orders_by_trip_id(&self,trip_id:&str) -> Vec<model::Order> {
        let coll = self.get_db().collection(model::Order::get_name());
        let mut doc = Document::new();
        doc.insert("trip_id",trip_id);
        match coll.find(Some(doc),None).map(|cursor|{
            cursor.map(|result| {
                result.map_err(|err|service::ServiceError::MongodbError(err)).and_then(|res|{
                    service::de_bson::<model::Order>(res)
                }).unwrap()
            }).collect()
        }) {
            Ok(result) => result,
            Err(err) => {
                warn!("get order by trip error : {}",err);
                Vec::new()
            }
        }
    }
   
    //db.Trip.update({"owner_id":"openid"},{"$set":{"status":"Finish"}})
    pub fn update_status(&self,id:&str,status:model::TripStatus) -> Result<()> {
        let coll = self.get_db().collection("Trip");
        let mut doc = Document::new();
        let mut update_doc = Document::new();
        let st:&str = &format!("{}",status);
        update_doc.insert("$set",doc!{"status" => st});
        oid::ObjectId::with_string(id).map_err(|err|service::ServiceError::BsonOidError(err)).map(|o|Bson::ObjectId(o)).and_then(|oid|{
            doc.insert("_id",oid);
            coll.update_many(doc,update_doc,None).map(|_|()).map_err(|err|service::ServiceError::MongodbError(err))
        })
    }

    fn get_db(&self) -> Database {
	    //let client = Client::connect("localhost", 27017)
        //.ok().expect("Failed to initialize standalone client.");
        let db_name = ConfigManager::get_config_str("app", "dbname");
        let db_user = ConfigManager::get_config_str("app", "dbuser");
        let db_pwd = ConfigManager::get_config_str("app", "dbpwd");
        let db = self.0.db(&db_name);
        db.auth(&db_user,&db_pwd).unwrap();
        db
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

fn get_db() -> Client {
	let client = Client::connect("localhost", 27017)
        .ok().expect("Failed to initialize standalone client.");
        //let db_name = ConfigManager::get_config_str("app", "dbname");
        //let db_user = ConfigManager::get_config_str("app", "dbuser");
        //let db_pwd = ConfigManager::get_config_str("app", "dbpwd");
        //let db = client.db(&db_name);
        //db.auth(&db_user,&db_pwd).unwrap();
        //db
        client
}

