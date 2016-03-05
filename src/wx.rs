use sha1;
use quickersort;
use iron::typemap::Key;
use iron::prelude::*;
use persistent::Read as PersistRead;
use persistent::State as PersistState;
use urlencoded::{UrlEncodedBody,UrlEncodedQuery};
use iron::{status,Url};
use iron::modifiers::Redirect;
use db::Dao;
use model::{self,Passenger,Owner,Trip};
use service::Service;
use mongodb::db::ThreadedDatabase;
use bodyparser;
use session::{Session,SessionContext};
use hbs::Template;
use chrono::UTC;
use chrono::offset::local::Local;
use chrono::offset::TimeZone;
use std::collections::HashMap;
use jsonway;

#[derive(Debug, Clone, RustcDecodable)]
struct MyStructure {
    a: String,
    b: Option<String>,
}
#[allow(dead_code)]
pub struct WxInstance {
	appid:       String,
	secret:      String,
	token:       String,
	access_token: String,
	open_id:      String,
}

//#[derive(Serialize, Deserialize, Debug)]
//#[cfg(feature = "serde_macros")]
/*
struct Message  {
	ToUserName:   String,
	FromUserName: String,
	CreateTime:   i64,
	MsgType:      String,
	//#[serde(skip_serializing_if_none)]
	Content:      String,
	//#[serde(skip_serializing_if_none)]
	PicUrl:       String,
	//#[serde(skip_serializing_if_none)]
	MediaId:      String,
	//#[serde(skip_serializing_if_none)]
	ThumbMediaId: String,
	//#[serde(skip_serializing_if_none)]
	Format:       String,
	//#[serde(skip_serializing_if_none)]
	Location_X:   String ,
	//#[serde(skip_serializing_if_none)]
	Location_Y:   String,
	//#[serde(skip_serializing_if_none)]
	Scale:        String,
	//#[serde(skip_serializing_if_none)]
	Label:        String,
	//#[serde(skip_serializing_if_none)]
	Title:        String,
	//#[serde(skip_serializing_if_none)]
	Description:  String,
	//#[serde(skip_serializing_if_none)]
	Url:          String,
	//#[serde(skip_serializing_if_none)]
	MsgId:        i64,
	//#[serde(skip_serializing_if_none)]
	ArticleCount: i64,
	//#[serde(skip_serializing_if_none)]
	Articles:     Vec<Item>,
}
*/
impl WxInstance {
	pub fn new() -> Self {
	    WxInstance{appid:"wxeb1110111c602545".to_string(),secret:"2c686564f1130b9cb3ed08d828388573".to_string(),token:"lp3385".to_string(),access_token:"String".to_string(),open_id:"String".to_string()}
	}

	pub fn check(&self,timestamp:&str, nonce:&str, echostr:&str, signature:&str) -> Result<String,&str> {
		let mut strs:Vec<&str> = vec![&self.token,nonce,timestamp];
		println!("strs is {:?}", strs);
		quickersort::sort(&mut strs[..]);
		let ss = strs.join("");
		let mut m = sha1::Sha1::new();
		m.reset();
        m.update(ss.as_bytes());
        let hh = m.hexdigest();
        println!("sha1 result is {}", hh);
        if &hh == signature {
        	Result::Ok(echostr.to_string())
        } else {
        	Result::Err("check error!")
        }
	}
}

impl Key for WxInstance { type Value = WxInstance; }


pub fn wx(req:&mut Request) -> IronResult<Response>{
    let instance = req.get::<PersistRead<WxInstance>>().unwrap();
    match req.get_ref::<UrlEncodedQuery>() {
        Ok(ref hashmap) => {
            let timestamp = &hashmap.get("timestamp").unwrap()[0];
            let nonce = &hashmap.get("nonce").unwrap()[0];
            let echostr = &hashmap.get("echostr").unwrap()[0];
            let signature = &hashmap.get("signature").unwrap()[0];
            match instance.check(timestamp,nonce,echostr,signature) {
                Ok(echo) => Ok(Response::with((status::Ok,echo))),
                Err(err) => Ok(Response::with((status::Ok,err)))
            }
        },
        Err(_) => Ok(Response::with((status::Ok,"error parameters!")))
    }
}

pub fn register_owner(req:&mut Request) -> IronResult<Response> {
	let service = req.get::<PersistRead<Service>>().unwrap();
	match req.get_ref::<UrlEncodedBody>() {
        Ok(ref hashmap) => {
            let name = &hashmap.get("name").unwrap()[0];
            let carType = &hashmap.get("car_type").unwrap()[0];
            let tel = &hashmap.get("tel").unwrap()[0];
            let code = &hashmap.get("code").unwrap()[0];
            let plate_number = &hashmap.get("plate_number").unwrap()[0];
            
            let mut owner = Owner::new(tel.to_owned(),carType.to_owned(),plate_number.to_owned());
            owner.name = Some(name.to_owned());
            service.add_owner(owner).unwrap();
            Ok(Response::with((status::Ok,"add owner sucess!")))
        },
        Err(_) => Ok(Response::with((status::Ok,"error parameters!")))
    }
}

pub fn publish_trip(req:&mut Request) -> IronResult<Response> {

    let mut can = false;

    if let Some(user) = get_session(req, "user_type") {
        if user == "owner" || user == "both" {
            can = true;
        }
    }

    if !can {
        return  Ok(Response::with((status::Ok,"you are not a owner ,can't publish Trip !")));
    }

    let open_id = get_session(req, "open_id").unwrap();

    let service = req.get::<PersistRead<Service>>().unwrap();
    match req.get_ref::<UrlEncodedBody>() {
        Ok(ref hashmap) => {
            let line_id = &hashmap.get("line_id").unwrap()[0];
            let start_time = &hashmap.get("start").unwrap()[0];
            let seat_count = &hashmap.get("seat_count").unwrap()[0];

            if let Ok(id) = line_id.parse::<u32>() {
                if let Ok(seat) = seat_count.parse::<u32>() {
                    if let Ok(start) = Local.datetime_from_str(start_time, "%Y-%m-%d %H:%M:%S") {
                        //start.with_timezone(&UTC);
                        let t = Trip::new(open_id,id,start,seat);
                        service.add_trip(t);
                        return Ok(Response::with((status::Ok,"publish Trip sucess!")));
                    }
                }
            }

            return Ok(Response::with((status::Ok,"error parameters!")));
        },
        Err(_) => Ok(Response::with((status::Ok,"error parameters!")))
    }
}

pub fn register_passenger(req:&mut Request) -> IronResult<Response> {
    let service = req.get::<PersistRead<Service>>().unwrap();
    match req.get_ref::<UrlEncodedBody>() {
        Ok(ref hashmap) => {
            let name = &hashmap.get("name").unwrap()[0];            
            let tel = &hashmap.get("tel").unwrap()[0];
            let code = &hashmap.get("code").unwrap()[0];
            
            let mut p = Passenger::new(tel.to_owned());
            p.name = Some(name.to_owned());

            service.add_passenger(p).unwrap();
            Ok(Response::with((status::Ok,"add Passenger sucess!")))
        },
        Err(_) => Ok(Response::with((status::Ok,"error parameters!")))
    }
}

pub fn get_trips(req:&mut Request) -> IronResult<Response> {
    let service = req.get::<PersistRead<Service>>().unwrap();
    Ok(Response::with((status::Ok,service.get_new_trips())))
}

pub fn login(req:&mut Request) -> IronResult<Response> {
    let service = req.get::<PersistRead<Service>>().unwrap();
    let mut open_id = String::new();
    if let Ok(ref hashmap) = req.get_ref::<UrlEncodedBody>() {
        open_id = hashmap.get("open_id").unwrap()[0].to_owned();            
    }
    if open_id.is_empty() {
        let s = r#"{"success":false,"msg":"parameters errror"}"#;
        Ok(Response::with((status::Ok,s)))
    } else {
        match service.get_user_by_id(&open_id) {
            (Some(o),None) => {
                let s = r#"{"success":true,"type":"owner"}"#;
                let mut res = Response::with((status::Ok,s));
                set_session(req, &mut res, "open_id".to_string(), o.open_id);
                set_session(req, &mut res, "name".to_string(), o.name.unwrap_or("anonymous".to_string()));
                set_session(req, &mut res, "user_type".to_string(), "owner".to_string());
                Ok(res)
            },
            (None,Some(p)) => {
                let s = r#"{"success":true,"type":"passenger"}"#;
                let mut res = Response::with((status::Ok,s));
                set_session(req, &mut res, "open_id".to_string(), p.open_id);
                set_session(req, &mut res, "name".to_string(), p.name.unwrap_or("anonymous".to_string()));
                set_session(req, &mut res, "user_type".to_string(), "passenger".to_string());
                Ok(res)
            },
            (Some(o),Some(p)) => {
                let s = r#"{"success":true,"type":"both"}"#;
                let mut res = Response::with((status::Ok,s));
                set_session(req, &mut res, "open_id".to_string(), p.open_id);
                set_session(req, &mut res, "name".to_string(), p.name.unwrap_or("anonymous".to_string()));
                set_session(req, &mut res, "user_type".to_string(), "both".to_string());
                Ok(res)
            },
            (None,None) => {
                let s = r#"{"success":false,"msg":"login faile!!!"}"#;
                Ok(Response::with((status::Ok,s)))
            }
        }
    }
}

pub fn apply_trip(req:&mut Request) -> IronResult<Response> {

    let mut can = false;
    if let Some(user) = get_session(req, "user_type") {
        if user == "passenger" || user == "both" {
            can = true;
        }
    }
    if !can {
        return  Ok(Response::with((status::Ok,"{success:false,login:false}")));
    }

    let open_id = get_session(req, "open_id").unwrap();
    let ip = format!("{}",req.remote_addr);
    let service = req.get::<PersistRead<Service>>().unwrap();
    match req.get_ref::<UrlEncodedBody>() {
        Ok(ref hashmap) => {
            let oid = &hashmap.get("oid").unwrap()[0];    
            if let Ok(payid) = service.apply_trip(oid,&open_id,ip) {
                let json_replay = jsonway::object(|j|{
                    j.set("success",true);
                    j.set("payid",payid.clone());
                });
                Ok(Response::with((status::Ok,format!("{}",payid))))
            } else {
                Ok(Response::with((status::Ok,"{success:false}")))
            }  
           
        },
        Err(_) => Ok(Response::with((status::Ok,"{success:false}")))
    }
}

pub fn test(req: &mut Request) -> IronResult<Response> {
    let mut resp = Response::new();

    // open http://localhost:3000/
    
    let data = model::make_data();
    info!("{:?}", data);
    resp.set_mut(Template::new("index", data)).set_mut(status::Ok);
    
    Ok(resp)
}

pub fn ico(req: &mut Request) -> IronResult<Response> {
    let urlstr = "http://127.0.0.1:6767/static/favicon.ico".to_owned();
    let mut response = Response::new();
    let url = Url::parse(&urlstr).unwrap();
    response.set_mut(status::Found).set_mut(Redirect(url));
    Ok(response)
}

pub fn set_session(req: &mut Request,res:&mut Response,key:String,value:String) {
    let mut sc1 = req.get::<PersistState<SessionContext>>().unwrap();
    let mut sc = sc1.write().unwrap();
    let mut has = false;
    {
        let mut session = sc.get_mut_session(req);
        if let Some(s) = session {
            s.data.insert(key.clone(),value.clone());
            has = true;
        }  
    }
    if !has {
        let s = sc.new_session(res);
        s.data.insert(key,value);
    }           
}

pub fn get_session(req: &mut Request,key:&str) -> Option<String> {
    let mut sc1 = req.get::<PersistState<SessionContext>>().unwrap();
    let mut sc = sc1.read().unwrap();
    let session = sc.get_session(req);
    if let Some(s) = session {
        if let Some(value) = s.data.get(key) {
            Some(value.to_owned())
        } else {
            None
        }
    } else {
        warn!("get session key {} error", key);
        None
    }
}

/*
pub fn test(req: &mut Request) -> IronResult<Response> {
	let mut sc1 = req.get::<PersistState<SessionContext>>().unwrap();
    let mut sc = sc1.write().unwrap();
	let session = sc.get_mut_session(req);
    if let Some(s) = session {
        let mut has = false;
    	if let Some(name) = s.data.get("name") {
    		println!("{}", name);
            has = true;
    	}
        if !has {
            s.data.insert("name".to_owned(),"liupeng".to_owned());
        }
    }
    Ok(Response::with((status::Ok,"i don't who are you "))) 
}
*/
/*
fn log_body(req: &mut Request) -> IronResult<Response> {
    let body = req.get::<bodyparser::Raw>();
    match body {
        Ok(Some(body)) => println!("Read body:\n{}", body),
        Ok(None) => println!("No body"),
        Err(err) => println!("Error: {:?}", err)
    }

    let json_body = req.get::<bodyparser::Json>();
    match json_body {
        Ok(Some(json_body)) => println!("Parsed body:\n{}", json_body),
        Ok(None) => println!("No body"),
        Err(err) => println!("Error: {:?}", err)
    }

    let struct_body = req.get::<bodyparser::Struct<MyStructure>>();
    match struct_body {
        Ok(Some(struct_body)) => println!("Parsed body:\n{:?}", struct_body),
        Ok(None) => println!("No body"),
        Err(err) => println!("Error: {:?}", err)
    }



    Ok(Response::with(status::Ok))
}
*/
