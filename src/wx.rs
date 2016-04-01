use sha1;
use quickersort;
use iron::typemap::Key;
use iron::prelude::*;
use persist::Read as PersistRead;
use persist::State as PersistState;
use urlencoded::{UrlEncodedBody,UrlEncodedQuery};
use iron::{status,Url};
use iron::modifiers::Redirect;
use db::Dao;
use model::{self,Passenger,Owner,Trip,ApiResult,LoginStatus,UserType,WxUserInfo};
use service::Service;
use mongodb::db::ThreadedDatabase;
use session::{Session,SessionContext};
use hbs::Template;
use chrono::UTC;
use chrono::offset::local::Local;
use chrono::offset::TimeZone;
use std::collections::HashMap;
use std::io::Read;
use std::marker::{Sync,Send};
use jsonway;
use pay;
use serde_json;
use hyper;
use config::ConfigManager;

#[derive(Debug, Clone, RustcDecodable)]
struct MyStructure {
    a: String,
    b: Option<String>,
}
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
            let line_id = &hashmap.get("line_id").unwrap()[0];
            let start_time = &hashmap.get("start").unwrap()[0];
            let seat_count = &hashmap.get("seat_count").unwrap()[0];

            if let Ok(id) = line_id.parse::<u32>() {
                if let Ok(seat) = seat_count.parse::<u32>() {
                    if let Ok(start) = Local.datetime_from_str(start_time, "%Y-%m-%d %H:%M:%S") {
                        //start.with_timezone(&UTC);
                        let t = Trip::new(login_status.openid.clone(),id,start,seat);
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

/*
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

    let mut can = false;
    let mut  login_status:LoginStatus = LoginStatus::default();
    if let Some(status) = get_session::<LoginStatus>(req) {
            can = true;
            login_status = status.clone();
    }
    if !can {
        return  Ok(Response::with((status::Ok,"{success:false,login:false}")));
    }
    let ip = format!("{}",req.remote_addr);
    let service = req.get::<PersistRead<Service>>().unwrap();
    match req.get_ref::<UrlEncodedBody>() {
        Ok(ref hashmap) => {
            let oid = &hashmap.get("oid").unwrap()[0];    
            if let Ok(payid) = service.apply_trip(oid,&login_status.openid,ip) {
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
    let domain = ConfigManager::get_config_str("app", "domain");
    let urlstr = domain+"/static/favicon.ico";
    redirect!(&urlstr)
}

pub fn index(req: &mut Request) -> IronResult<Response> {
    let domain = ConfigManager::get_config_str("app", "domain");
    let urlstr = domain+"/static/index.html";
    redirect!(&urlstr)
}

pub fn index_template(req: &mut Request) -> IronResult<Response> {
    let data = model::make_data();

    let mut code = String::new();

    match req.get_ref::<UrlEncodedQuery>() {
        Ok(ref hashmap) => {
            code = hashmap.get("code").unwrap()[0].clone();
        },
        Err(_) => return Ok(Response::with((status::Ok,"error parameters!")))
    };
    let appid = ConfigManager::get_config_str("app", "appid");
    let secret = ConfigManager::get_config_str("app", "appsecret");

    let mut resp = Response::new();

    let client = pay::ssl_client();
    let url = format!("https://api.weixin.qq.com/sns/oauth2/access_token?appid={}&secret={}&code={}&grant_type=authorization_code",appid,secret,code);
    warn!("url is {}",url);
    client.get(&url).send().and_then(|mut res|{
        let mut buf = String::new();
        res.read_to_string(& mut buf).map(|_| buf).map_err(|err|hyper::Error::Io(err))
    }).and_then(|buf|{
        serde_json::from_str::<ApiResult>(&buf).map_err(|err| hyper::Error::Method)
    }).ok().and_then(|res|{  
           if let Some(token) = res.access_token {
                let mut login_status = LoginStatus::default();
                login_status.web_token = Some(token);
                login_status.refresh_token = res.refresh_token;
                login_status.openid = res.openid.unwrap_or(String::new());
                warn!("{:?}",login_status);
                set_session::<LoginStatus>(req, &mut resp, login_status);
                Some(())
           } else {
                 warn!("get access token error!!");
                 None
           }
    });

    res_template!("index",data,resp)
}

pub fn my_info_template(req: &mut Request) -> IronResult<Response> {
    let mut resp = Response::new();
    let login_status = get_session::<LoginStatus>(req).unwrap();
    warn!("{:?}",login_status);
    let user_info = get_wx_user(&login_status.web_token.unwrap(), &login_status.openid);
    warn!("{:?}",user_info);
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

