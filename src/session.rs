use hyper::header::{Headers, Cookie,SetCookie};
use cookie::Cookie as CookieObj;
use std::collections::HashMap;
use std::marker::{Sync,Send};
use iron::prelude::*;
use iron::{status,AfterMiddleware};
use iron::modifiers::Header;
use iron::typemap::{TypeMap, Key};
use persist::State as PersistState;

use uuid::Uuid;


pub struct Session{
    pub data: TypeMap,
    expire: u32
}

pub struct CheckSession;

pub struct SessionContext(pub HashMap<String,Session>);

impl Key for SessionContext {
    type Value = SessionContext;
}

unsafe impl Sync for SessionContext {
    // add code here
}

unsafe impl Send for SessionContext {
    // add code here
}

impl SessionContext {
    pub fn get_session(&self,req:& Request) -> Option<&Session> {
      	let cookie = req.headers.get::<Cookie>();
    	match cookie {
            Some(ref value) => {
	    		let Cookie(ref ckvec) = **value;
                let cookie_vec = ckvec.iter()
                                    .filter(|item: &&CookieObj| item.name == "sessionid".to_owned())
                                    .take(1)
                                    .collect::<Vec<&CookieObj>>();
                let cookie_obj = cookie_vec[0];
                let cookie_value = cookie_obj.value.clone();
                //let SessionContext(data) = *self;
                self.0.get(&cookie_value)

            }
            None => None
        }
    }

    pub fn get_mut_session(&mut self,req:&mut Request) -> Option<&mut Session> {
      	let cookie = req.headers.get::<Cookie>();
    	match cookie {
            Some(ref value) => {
	    		let Cookie(ref ckvec) = **value;
                let cookie_vec = ckvec.iter()
                                    .filter(|item: &&CookieObj| item.name == "sessionid".to_owned())
                                    .take(1)
                                    .collect::<Vec<&CookieObj>>();
                let cookie_obj = cookie_vec[0];
                let cookie_value = cookie_obj.value.clone();
                //let SessionContext(data) = *self;
                self.0.get_mut(&cookie_value)

            }
            None => None
        }
    }

    pub fn new_session(&mut self,res:&mut Response) -> &mut Session{
    	let uid = Uuid::new_v4().to_simple_string();
    	let mut cookie = CookieObj::new("sessionid".to_string(),uid.clone());
    	cookie.path = Some("/".to_owned());
        res.set_mut(Header(SetCookie(vec![cookie])));
    	
    	let session = Session { data:TypeMap::new(),expire:0u32 };
    	self.0.insert(uid.clone(),session);
        self.0.get_mut(&uid).unwrap()
    }
}

impl AfterMiddleware for CheckSession {
    fn after(&self, req: &mut Request, mut res: Response) -> IronResult<Response> {
    	let sc1 = req.get::<PersistState<SessionContext>>().unwrap();
	    let mut has = false;
	    {
	    	let sc = sc1.read().unwrap();
	    	let session = sc.get_session(req);
		    match session {
		        Some(_) => {
		            //println!("{:?}", s);
		            has = true;
		        }
		        None => {
		            has = false;
		            //println!("none");
		        }
		    }
	    }
	    
	    if !has {
	    	let mut sc = sc1.write().unwrap();
	    	sc.new_session(&mut res);
	    }
	    Ok(res)
    }
}