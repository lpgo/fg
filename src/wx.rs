use sha1;
use quickersort;
use iron::typemap::Key;
use iron::prelude::*;
use persist::Read as PersistRead;
use persist::State as PersistState;
use urlencoded::{UrlEncodedBody,UrlEncodedQuery};
use iron::{status,Url};
use iron::modifiers::Redirect;
use iron::error::HttpError;
use db::Dao;
use model::{self,Passenger,Owner,Trip,ApiResult,LoginStatus,UserType,WxUserInfo,TripStatus};
use service::{Service,ServiceError};
use mongodb::db::ThreadedDatabase;
use session::{Session,SessionContext};
use hbs::Template;
use chrono::UTC;
use chrono::offset::local::Local;
use chrono::offset::TimeZone;
use std::collections::{HashMap,BTreeMap};
use std::io::Read;
use std::result;
use std::marker::{Sync,Send};
use jsonway;
use pay;
use serde_json;
use hyper;
use config::ConfigManager;
use rustc_serialize::json;

pub type Result<T> = result::Result<T, ServiceError>;

#[derive(Clone,Debug)]
pub struct WxInstance {
	appid:       String,
	secret:      String,
	token:       String,
	pub access_token: String,
             pub access_token_expires:u32,
	openid:      String,
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
                let appid = ConfigManager::get_config_str("app","appid");
                let secret = ConfigManager::get_config_str("app","appsecret");
                let token = ConfigManager::get_config_str("app","token");
	   let mut instance =  WxInstance{appid:appid,secret:secret,token:token,access_token:String::new(),access_token_expires:0u32,openid:String::new()};
                instance.get_access_token();
                instance
	}

	pub fn check(&self,timestamp:&str, nonce:&str, echostr:&str, signature:&str) -> Result<String> {
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
                        	Ok(echostr.to_string())
                        } else {
                        	Err(ServiceError::Other("check error!".to_string()))
                        }
	}

            pub fn get_access_token(& mut self){
                let client = pay::ssl_client();
                let url = format!("https://api.weixin.qq.com/cgi-bin/token?grant_type=client_credential&appid={}&secret={}",self.appid,self.secret);
                client.get(&url).send().and_then(|mut res|{
                    let mut buf = String::new();
                    res.read_to_string(& mut buf).map(|_| buf).map_err(|err|hyper::Error::Io(err))
                }).and_then(|buf|{
                    serde_json::from_str::<ApiResult>(&buf).map_err(|err| hyper::Error::Method)
                }).ok().and_then(|res|{  
                       if let Some(token) = res.access_token {
                            self.access_token = token;
                            let expires = res.expires_in.unwrap_or(7200u32);
                            self.access_token_expires = expires;
                            Some(expires)
                       } else {
                             warn!("get access token error!!");
                             None
                       }
                });
            }

           
            pub fn get_user_list(&self){
                let client = pay::ssl_client();
                let url = format!("https://api.weixin.qq.com/cgi-bin/user/get?access_token={}",self.access_token);
                warn!("url is {}",url);
                client.get(&url).send().and_then(|mut res|{
                    let mut buf = String::new();
                    res.read_to_string(& mut buf).map(move |_| buf).map_err(|err|hyper::Error::Io(err))
                }).and_then(|buf|{
                    warn!("userList is {}",buf);
                    Ok(buf)
                });
            }

            pub fn get_user_info(&self,openid:&str){
                let client = pay::ssl_client();
                let url = format!("https://api.weixin.qq.com/cgi-bin/user/info?access_token={}&openid={}&lang=zh_CN",self.access_token,openid);
                client.get(&url).send().and_then(|mut res|{
                    let mut buf = String::new();
                    res.read_to_string(& mut buf).map(move |_| buf).map_err(|err|hyper::Error::Io(err))
                }).and_then(|buf|{
                    warn!("{} 's info is {}",openid,buf);
                    Ok(buf)
                });
            }

}

impl Key for WxInstance { type Value = WxInstance; }


pub fn wx(req:&mut Request) -> IronResult<Response>{
    let instance1 = req.get::<PersistState<WxInstance>>().unwrap();
    let instance = instance1.read().unwrap();
    match req.get_ref::<UrlEncodedQuery>() {
        Ok(ref hashmap) => {
            let timestamp = &hashmap.get("timestamp").unwrap()[0];
            let nonce = &hashmap.get("nonce").unwrap()[0];
            let echostr = &hashmap.get("echostr").unwrap()[0];
            let signature = &hashmap.get("signature").unwrap()[0];
            match instance.check(timestamp,nonce,echostr,signature) {
                Ok(echo) => Ok(Response::with((status::Ok,echo))),
                Err(_) => Ok(Response::with((status::Ok,"check error")))
            }
        },
        Err(_) => Ok(Response::with((status::Ok,"error parameters!")))
    }
}

pub fn register_owner(req:&mut Request) -> IronResult<Response> {
        let service = req.get::<PersistRead<Service>>().unwrap();
        let mut login_status = get_session::<LoginStatus>(req).unwrap();
   
        req.get_ref::<UrlEncodedBody>().and_then(|hashmap|{
            let plate_number = &hashmap.get("plate_number").unwrap()[0];
            let car_type = &hashmap.get("car_type").unwrap()[0];
            let openid = login_status.openid.clone();
            let p = login_status.passenger.as_ref().unwrap();
            let owner = Owner::new(p.tel.clone(),car_type.clone(),plate_number.clone(),openid);
            service.add_owner(owner.clone());
            login_status.owner = Some(owner);
            Ok(())
        });

        let mut resp = Response::new();
        let ls = login_status.clone();
        set_session::<LoginStatus>(req, &mut resp, login_status);
        res_template!("publishTrip",ls.owner.unwrap(),resp)
}

pub fn can_publish_trip(req:&mut Request) -> IronResult<Response> {
    get_session::<LoginStatus>(req).ok_or(IronError::new(HttpError::Method,"can not get session")).and_then(|ls|{
        match ls.user_type {
            UserType::Owner => {
                let mut resp = Response::new();
                res_template!("publishTrip",ls.owner.unwrap(),resp)
            },
            UserType::Passenger => redirect!("/static/driverregister.html"),
            UserType::Anonymous => redirect!("/static/confirmation.html"),
        }
    })
}

pub fn publish_trip(req:&mut Request) -> IronResult<Response> {

    let mut can = false;
    let mut  login_status:LoginStatus = LoginStatus::default();
    if let Some(user) = get_session::<LoginStatus>(req) {
        if user.user_type == UserType::Owner {
            can = true;
        }
        login_status = user.clone();
    }

    if !can {
        return  Ok(Response::with((status::Ok,"you are not a owner ,can't publish Trip !")));
    }

    let service = req.get::<PersistRead<Service>>().unwrap();
    match req.get_ref::<UrlEncodedBody>() {
        Ok(ref hashmap) => {
            let line_id = &hashmap.get("lineId").unwrap()[0];
            let start_time = &hashmap.get("startTime").unwrap()[0];
            let seat_count = &hashmap.get("seatCount").unwrap()[0];
            let venue = &hashmap.get("venue").unwrap()[0];

            warn!("time type is  {}",start_time);

            if let Ok(id) = line_id.parse::<u32>() {
                if let Ok(seat) = seat_count.parse::<u32>() { 
                    if let Ok(start) = Local.datetime_from_str(start_time, "%Y-%m-%dT%H:%M") {
                        //start.with_timezone(&UTC);
                        let mut  t = Trip::default();
                        let line = service.get_line_by_id(id);
                        match line {
                            Ok(line) => {
                                t.owner_id = login_status.openid.clone();
                                t.line_id = id;
                                t.start = line.start;
                                t.end = line.end;
                                t.price = format!("{:.*}",2,line.price as f32/100f32);
                                t.start_time = start.timestamp();
                                t.start_time_text = start.format("%Y-%m-%d %H:%M").to_string();
                                t.seat_count = seat;
                                t.current_seat = seat;
                                t.status = TripStatus::Prepare.to_string();
                                t.venue = venue.clone();
                                service.add_trip(t);
                                return Ok(Response::with((status::Ok,"publish Trip sucess!")));
                            },
                            Err(err) => {
                                warn!("get line has a err :{}",err);
                                return Ok(Response::with((status::Ok,"err line id")));
                            }
                        }
                    }
                }
            }

            return Ok(Response::with((status::Ok,"error parameters!")));
        },
        Err(_) => Ok(Response::with((status::Ok,"error parameters!")))
    }
}

pub fn trip_detail_template(req:&mut Request) -> IronResult<Response> {
    match req.get::<PersistRead<Service>>().map_err(|err|ServiceError::PersistentError(err)).and_then(|service|{
        req.get_ref::<UrlEncodedBody>().map_err(|err|ServiceError::UrlDecodingError(err)).and_then(|hashmap|{
            let oid = &hashmap.get("oid").unwrap()[0];
            service.get_trip_by_oid(oid)
        })
    }) {
            Ok(trip) => {
                let mut resp = Response::new();
                res_template!("tripDetail",trip,resp)
                //Ok(Response::with((status::Ok,format!("get trip detail has error : {:?}",trip))))
            },
            Err(err) => {
                Ok(Response::with((status::Ok,format!("get trip detail has error : {}",err))))
            }
    }
}

pub fn register_passenger(req:&mut Request) -> IronResult<Response> {
    let service = req.get::<PersistRead<Service>>().unwrap();
    let mut login_status = get_session::<LoginStatus>(req).unwrap();
    let mut success = false;
    let mut to_owner = false;
    match req.get_ref::<UrlEncodedBody>() {
        Ok(ref hashmap) => {       
            let tel = &hashmap.get("tel").unwrap()[0];
            let code = &hashmap.get("code").unwrap()[0];
            let owner = &hashmap.get("owner").unwrap()[0];
            //to-do validate code 
            let mut p = Passenger::new(tel.to_owned(),login_status.openid.clone());
            service.add_passenger(p).unwrap();
            success = true;
            if !owner.is_empty() {
                to_owner = true;
            }
        },
        Err(_) => {}
    };
    if success {
        login_status.user_type  = UserType::Passenger;
        let mut resp = Response::new();
        set_session::<LoginStatus>(req, &mut resp, login_status);
        let data = model::make_data();
        if to_owner {
            redirect!("/static/driverregister.html")
        } else {
            res_template!("index",data,resp)
        }
    } else  {
        Ok(Response::with((status::Ok,"validate passenger faile")))
    }
    
}
/*
pub fn get_trips(req:&mut Request) -> IronResult<Response> {
    let service = req.get::<PersistRead<Service>>().unwrap();
    Ok(Response::with((status::Ok,service.get_new_trips())))
}


pub fn login(req:&mut Request) -> IronResult<Response> {
    let service = req.get::<PersistRead<Service>>().unwrap();
    let mut openid = String::new();
    if let Ok(ref hashmap) = req.get_ref::<UrlEncodedBody>() {
        openid = hashmap.get("openid").unwrap()[0].to_owned();            
    }
    if openid.is_empty() {
        let s = r#"{"success":false,"msg":"parameters errror"}"#;
        Ok(Response::with((status::Ok,s)))
    } else {
        match service.get_user_by_id(&openid) {
            (Some(o),None) => {
                let s = r#"{"success":true,"type":"owner"}"#;
                let mut res = Response::with((status::Ok,s));
                let login_status = LoginStatus{openid:o.openid,user_type:UserType::Owner,name:o.name};
                set_session::<LoginStatus>(req, &mut res, login_status);
                Ok(res)
            },
            (None,Some(p)) => {
                let s = r#"{"success":true,"type":"passenger"}"#;
                let mut res = Response::with((status::Ok,s));
                let login_status = LoginStatus{openid:p.openid,user_type:UserType::Passenger,name:p.name};
                set_session::<LoginStatus>(req, &mut res, login_status);
                Ok(res)
            },
            (Some(o),Some(p)) => {
                let s = r#"{"success":true,"type":"owner"}"#;
                let mut res = Response::with((status::Ok,s));
                let login_status = LoginStatus{openid:o.openid,user_type:UserType::Owner,name:o.name};
                set_session::<LoginStatus>(req, &mut res, login_status);
                Ok(res)
            },
            (None,None) => {
                let s = r#"{"success":false,"msg":"login faile!!!"}"#;
                Ok(Response::with((status::Ok,s)))
            }
        }
    }
}
*/
pub fn apply_trip(req:&mut Request) -> IronResult<Response> {

    let ip = format!("{}",req.remote_addr);

    let replay = get_session::<LoginStatus>(req).and_then(|login_status|{
        if login_status.user_type != UserType::Anonymous {
            Some(login_status)
        } else {
            None
        }
    }).ok_or(ServiceError::NoLogin).and_then(|login_status|{
        req.get::<PersistRead<Service>>().map(|service|(service,login_status)).map_err(|err|ServiceError::PersistentError(err))
    }).and_then(|(service,login_status)|{
        req.get_ref::<UrlEncodedBody>().map(|hashmap|(service,login_status,hashmap)).map_err(|err|ServiceError::UrlDecodingError(err))
    }).and_then(|(service,login_status,hashmap)|{
        let oid  = &hashmap.get("oid").unwrap()[0];
        service.apply_trip(oid,&login_status.openid,ip)
    }).map(|payid|{
         jsonway::object(|j|{
                    j.set("success",true);
                    j.set("payid",payid);
            })
    });

    match replay {
        Ok(rep) => {
            let r = json::encode(&rep.unwrap()).unwrap();
            Ok(Response::with((status::Ok,format!("{}",r))))
        },
        Err(err) => {
                        warn!("{}",err);
                        Ok(Response::with((status::Ok,"{success:false}")))
        }
    }
}

pub fn get_lines(req: &mut Request) -> IronResult<Response> {
    let service = req.get::<PersistRead<Service>>().unwrap();
    Ok(Response::with((status::Ok,service.get_lines())))
}

pub fn get_hot_lines(req: &mut Request) -> IronResult<Response> {
    let service = req.get::<PersistRead<Service>>().unwrap();
    Ok(Response::with((status::Ok,service.get_hot_lines())))
}

pub fn test(req: &mut Request) -> IronResult<Response> {
    let data = model::make_data();
    let mut resp = Response::new();
    res_template!("index",data,resp)
}

pub fn ico(req: &mut Request) -> IronResult<Response> {
    redirect!("/static/favicon.ico")
}

pub fn index(req: &mut Request) -> IronResult<Response> {
    redirect!("/static/index.html")
}

pub fn index_template(req: &mut Request) -> IronResult<Response> {
    let mut resp = Response::new();
    req.get_ref::<UrlEncodedQuery>().map_err(|err|ServiceError::UrlDecodingError(err)).map(|hashmap|{
        &hashmap.get("code").unwrap()[0]
    }).and_then(|code|{
        get_web_token(code)        
    }).and_then(|api_result|{
        let mut login_status = LoginStatus::default();
        login_status.web_token = api_result.access_token;
        login_status.refresh_token = api_result.refresh_token;
        //login_status.openid = api_result.openid.unwrap_or(String::new());

        api_result.openid.and_then(|openid|{
            login_status.openid = openid.clone();
            req.get::<PersistRead<Service>>().ok().map(|service|(service,openid))
        }).and_then(|(service,openid)|{
            let (o,p) = service.get_user_by_id(&openid);
            p.map(|passenger|{
                login_status.user_type = UserType::Passenger;
                login_status.passenger = Some(passenger);
            });
            o.map(|owner|{
                login_status.user_type = UserType::Owner;
                login_status.owner = Some(owner);
            });
            Some(())
        });

        set_session::<LoginStatus>(req, &mut resp, login_status);
        Ok(())
    });
    match req.get::<PersistRead<Service>>().map(|service|{
        service.get_new_trips()
    }) {
        Ok(vec) => {
            warn!("data is ---  {:?}",vec);
            warn!("data json is {}",serde_json::to_string(&vec).unwrap());
            let mut data = BTreeMap::new();
            data.insert("vec",vec);
            res_template!("index",data,resp)
        },
        Err(err) => {
            warn!("get serivce err :{}",err);
            let data:Vec<model::Trip> = Vec::new();
            res_template!("index",data,resp)
        } 
    }
}

pub fn get_web_token(code:&str) -> Result<ApiResult> {
    let appid = ConfigManager::get_config_str("app", "appid");
    let secret = ConfigManager::get_config_str("app", "appsecret");

    let client = pay::ssl_client();
    let url = format!("https://api.weixin.qq.com/sns/oauth2/access_token?appid={}&secret={}&code={}&grant_type=authorization_code",appid,secret,code);
    client.get(&url).send().map_err(|err|ServiceError::HyperError(err)).and_then(|mut res|{
        let mut buf = String::new();
        res.read_to_string(& mut buf).map(|_| buf).map_err(|err|ServiceError::IoError(err))
    }).and_then(|buf|{
        serde_json::from_str::<ApiResult>(&buf).map_err(|err| ServiceError::SerdeJsonError(err))
    })
}


pub fn my_info_template(req: &mut Request) -> IronResult<Response> {
    let mut resp = Response::new();
    let login_status = get_session::<LoginStatus>(req).unwrap();
    let user_info = get_wx_user(&login_status.web_token.unwrap(), &login_status.openid);
    res_template!("profile",user_info,resp)
}


pub fn get_wx_user(token:&str,openid:&str) -> WxUserInfo {
    let client = pay::ssl_client();
    let url = format!("https://api.weixin.qq.com/sns/userinfo?access_token={}&openid={}&lang=zh_CN",token,openid);
    client.get(&url).send().and_then(|mut res|{
        let mut buf = String::new();
        res.read_to_string(& mut buf).map(|_| buf).map_err(|err|hyper::Error::Io(err))
    }).and_then(|buf|{
        serde_json::from_str::<WxUserInfo>(&buf).map_err(|err| hyper::Error::Method)
    }).unwrap()
}



pub fn set_session<K:Key>(req: &mut Request,res:&mut Response,value:K::Value) where K::Value:Clone{
    let mut sc1 = req.get::<PersistState<SessionContext>>().unwrap();
    let mut sc = sc1.write().unwrap();
    let mut has = false;
    {
        let mut session = sc.get_mut_session(req);
        if let Some(s) = session {
            s.data.insert::<K>(value.clone());
            has = true;
        }  
    }
    if !has {
        let s = sc.new_session(res);
        s.data.insert::<K>(value);
    }           
}

pub fn get_session<K:Key>(req: & mut Request) -> Option<K::Value> where K::Value:Clone {
    let mut sc1 = req.get::<PersistState<SessionContext>>().unwrap();
    let sc = sc1.read().unwrap();
    let session = sc.get_session(req);
    if let Some(s) = session {
        s.data.get::<K>().map(|v| v.clone())
    } else {
        warn!("get session key  error");
        None
    }
}

/*
pub fn get_mut_session<'a,K:Key>(req: &'a mut Request) -> Option<&'a mut K::Value> where K::Value:Clone {
    let mut sc1 = req.get::<PersistState<SessionContext>>().unwrap();
    let sc = sc1.read().unwrap();
    let session = sc.get_mut_session(req);
    session.and_then(|s|{
        s.data.get_mut::<K>()
    })
}
*/

