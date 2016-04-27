use hyper;
use hyper::client::{Client,Pool};
use hyper::net::HttpsConnector;
use hyper::net::Openssl;
use model::PrePayResult;
use service::de_xml;
use std::io::Read;
use std::default::Default;
use std::sync::Arc;
use std::path::Path;
use std::collections::BTreeMap;
use uuid::Uuid;
use md5;
use config::ConfigManager;
use jsonway;
use chrono;
use url;

use openssl::ssl::{Ssl, SslContext, SslStream, SslMethod, SSL_VERIFY_NONE};
use openssl::ssl::error::StreamError as SslIoError;
use openssl::ssl::error::SslError;
use openssl::x509::X509FileType;	

//微信统一下单数据结构--https://pay.weixin.qq.com/wiki/doc/api/jsapi.php?chapter=9_1
pub struct PrePay {
    appid : String,
    mch_id : String,
    device_info : Option<String>,
    nonce_str : String,
    sign : String,
    body : String,
    detail : Option<String>,
    attach : String,
    out_trade_no : String,
    fee_type : Option<String>,
    total_fee : u32,
    spbill_create_ip : String,
    time_start : Option<String>,
    time_expire : Option<String>,
    goods_tag : Option<String>,
    notify_url : String,
    trade_type : String,
    product_id : Option<String>,
    limit_pay : Option<String>,
    openid : String
}

impl PrePay {
	
	pub fn new(order_id:String,trip_id:String,msg:String,openid:String,fee:u32) -> PrePay {
		let domain = ConfigManager::get_config_str("app", "domain");
		let appid = ConfigManager::get_config_str("app", "appid");
		let mch_id = ConfigManager::get_config_str("app", "mchid");
		PrePay{appid : appid,
		    mch_id : mch_id,
		    device_info : None,
		    nonce_str : order_id.clone(),
		    sign : String::new(),
		    body : msg,
		    detail : None,
		    attach : trip_id,
		    out_trade_no : order_id,
		    fee_type : None,
		    total_fee : fee,
		    spbill_create_ip : String::new(),
		    time_start : None,
		    time_expire : None,
		    goods_tag : None,
		    notify_url : domain+"/payResult",
		    trade_type : "JSAPI".to_string(),
		    product_id : None,
		    limit_pay : None,
		    openid : openid
		}
	}

    fn to_xml(&self) -> String {
    	let api_key = ConfigManager::get_config_str("app", "apikey");
    	let fee = format!("{}",self.total_fee);
    	let mut strs:BTreeMap<&str,&str> = BTreeMap::new();
	strs.insert("appid",&self.appid);
	strs.insert("body",&self.body);
	strs.insert("attach", &self.attach);
	strs.insert("mch_id",&self.mch_id);
	strs.insert("nonce_str",&self.nonce_str);
	strs.insert("notify_url",&self.notify_url);
	strs.insert("openid",&self.openid);
	strs.insert("out_trade_no",&self.out_trade_no);
	strs.insert("spbill_create_ip","192.168.1.1");
	strs.insert("total_fee",&fee);
	strs.insert("trade_type","JSAPI");
	let mut ss = String::new();
	for (k,v) in strs {
		ss.push_str(k);
		ss.push('=');
		ss.push_str(v);
		ss.push('&');
	}
	ss.push_str("key=");
	ss.push_str(&api_key);
	warn!("ss is {}",&ss);
	let sign = to_md5(&ss);

    	format!(r#"
		    		<xml>
					   <appid>{appid}</appid>
					   <attach>{attach}</attach>
					   <body>{body}</body>
					   <mch_id>{mch_id}</mch_id>
					   <nonce_str>{nonce_str}</nonce_str>
					   <notify_url>{notify_url}</notify_url>
					   <openid>{openid}</openid>
					   <out_trade_no>{out_trade_no}</out_trade_no>
					   <spbill_create_ip>192.168.1.1</spbill_create_ip>
					   <total_fee>{total_fee}</total_fee>
					   <trade_type>JSAPI</trade_type>
					   <sign>{sign}</sign>
					</xml>
		    	"#,appid=self.appid,body=self.body,mch_id=self.mch_id,nonce_str=self.nonce_str,
		    	notify_url=self.notify_url,openid=self.openid,out_trade_no=self.out_trade_no,
		    	total_fee=self.total_fee,sign=sign,attach=self.attach)
    }
}

pub fn ssl_client() -> Client {

	let mut ctx = SslContext::new(SslMethod::Sslv23).unwrap();
	ctx.set_cipher_list("DEFAULT").unwrap();

	let cert = Path::new("cert/cert.pem");
	let key = Path::new("cert/key.pem");
	let ca = Path::new("cert/ca.pem");

	ctx.set_certificate_file(&cert, X509FileType::PEM);
	ctx.set_private_key_file(&key, X509FileType::PEM);
	ctx.set_CA_file(&ca);
	ctx.set_verify(SSL_VERIFY_NONE, None);
	let https = HttpsConnector::new(Openssl { context: Arc::new(ctx) });
	let pool = Pool::with_connector(Default::default(),https);
	return Client::with_connector(pool);
}

pub fn pre_pay(p : PrePay) -> Result<PrePayResult,&'static str> {
	//let client = Client::new();
	let client = ssl_client();
	let xml = p.to_xml();
	warn!("send ** {}",&xml);
	if let Ok(ref mut res) = client.post("https://api.mch.weixin.qq.com/pay/unifiedorder").body(&xml).send() {
		let mut buf = String::new();
		if let Ok(_) = res.read_to_string(& mut buf) {
			warn!("{}", buf);
			if let Ok(prepay_resutl) = de_xml::<PrePayResult>(&buf) {
				return Ok(prepay_resutl);
			}
		}
	}
	Err("prepay error!!!")  
}

pub fn pay_to_client(openid:&str,amount:&str) {

	let nonce_str = Uuid::new_v4().to_simple_string();
	let api_key = ConfigManager::get_config_str("app", "apikey");
	let appid = ConfigManager::get_config_str("app", "appid");
	let mchid = ConfigManager::get_config_str("app", "mchid");
	let mut strs:BTreeMap<&str,&str> = BTreeMap::new();
	strs.insert("mch_appid",&appid);
	strs.insert("mchid",&mchid);
	strs.insert("nonce_str",&nonce_str);
	strs.insert("partner_trade_no",&nonce_str);
	strs.insert("openid",openid);
	strs.insert("check_name","NO_CHECK");
	strs.insert("amount",amount);
	strs.insert("desc","thank you");
	strs.insert("spbill_create_ip","192.168.1.1");
	let mut ss = String::new();
	for (k,v) in strs {
		ss.push_str(k);
		ss.push('=');
		ss.push_str(v);
		ss.push('&');
	}
	ss.push_str("key=");
	ss.push_str(&api_key);
	let sign = to_md5(&ss);

	let xml = format!(r#"
		<xml>
			<mch_appid>{}</mch_appid>
			<mchid>{}</mchid>
			<nonce_str>{}</nonce_str>
			<partner_trade_no>{}</partner_trade_no>
			<openid>{}</openid>
			<check_name>NO_CHECK</check_name>
			<amount>{}</amount>
			<desc>thank you</desc>
			<spbill_create_ip>192.168.1.1</spbill_create_ip>
			<sign>{}</sign>
		</xml>
	"#,&appid,&mchid,&nonce_str,&nonce_str,openid,amount,sign);

	warn!("xml is {}",xml);

	let client = ssl_client();
             let url = "https://api.mch.weixin.qq.com/mmpaymkttransfers/promotion/transfers";
            client.post(url).body(&xml).send().and_then(|mut res|{
                let mut buf = String::new();
                res.read_to_string(& mut buf).map(move |_| buf).map_err(|err|hyper::Error::Io(err))
            }).and_then(|buf|{
                warn!("pay clinet result is {}",buf);
                Ok(buf)
            });
}

pub fn create_pay_json(prepay_id:&str) -> jsonway::ObjectBuilder{
	let api_key = ConfigManager::get_config_str("app", "apikey");
	let appid = ConfigManager::get_config_str("app", "appid");
	let time = format!("{}",chrono::Local::now().timestamp());
	let nonce_str = Uuid::new_v4().to_simple_string();
	let package = format!("prepay_id={}",prepay_id);
	let mut sign = String::new();
	{
    		let mut strs:BTreeMap<&str,&str> = BTreeMap::new();
    		strs.insert("appId",&appid);
		strs.insert("timeStamp",&time);
		strs.insert("nonceStr",&nonce_str);
		strs.insert("package",&package);
		strs.insert("signType","MD5");
		let mut ss = String::new();
		for (k,v) in strs {
			ss.push_str(k);
			ss.push('=');
			ss.push_str(v);
			ss.push('&');
		}
		ss.push_str("key=");
		ss.push_str(&api_key);
		sign= to_md5(&ss);
	}
	 jsonway::object(|j|{
	      j.set("success","true".to_string());
                    j.set("appId",appid);
                    j.set("timeStamp",time);
                    j.set("nonceStr",nonce_str);
                    j.set("package",package);
                    j.set("signType","MD5".to_string());
                    j.set("paySign",sign);
            })
}

pub fn send_sms() {
	let key = ConfigManager::get_config_str("app", "alikey");
	let secret = ConfigManager::get_config_str("app", "alisecret");
	let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
	let mut content = String::new();
	{
    		let mut strs:BTreeMap<&str,&str> = BTreeMap::new();
    		strs.insert("method","alibaba.aliqin.fc.sms.num.send");
		strs.insert("app_key",&key);
		strs.insert("timestamp",&now);
		strs.insert("v","2.0");
		strs.insert("sign_method","md5");
		strs.insert("sms_type","normal");
		strs.insert("sms_free_sign_name","身份验证");
		strs.insert("rec_num","18681926648");
		strs.insert("sms_template_code","SMS_7425163");
		strs.insert("sms_param","{\"code\":\"4444\",\"product\":\"ttpc\"}");
		let mut ss = String::new();
		ss.push_str(&secret);
		for (k,v) in strs.clone() {
			ss.push_str(k);
			ss.push_str(v);
		}
		ss.push_str(&secret);
		warn!("sign is {}",ss);
		let sign= to_md5(&ss);

		let mut encode = url::form_urlencoded::Serializer::new(String::new());
		for (k,v) in strs {
			encode.append_pair(k, v);
		}
		encode.append_pair("sign",&sign);
		content = encode.finish();
	}
	warn!("content : {}",content);
	let client = ssl_client();
             	let url = "https://eco.taobao.com/router/rest";
            	client.post(url).header(hyper::header::ContentType::form_url_encoded()).body(&content).send().and_then(|mut res|{
                	let mut buf = String::new();
                	res.read_to_string(& mut buf).map(move |_| buf).map_err(|err|hyper::Error::Io(err))
           	 }).and_then(|buf|{
                	warn!("pay clinet result is {}",buf);
                	Ok(buf)
            	});
}

pub fn to_md5(s:&str) -> String {
	let mut context = md5::Context::new();
            context.consume(s.as_bytes());
            let mut digest = String::with_capacity(2 * 16);
            for x in &context.compute()[..] {
                digest.push_str(&format!("{:02x}", x));
            }
            digest.to_uppercase()
}



