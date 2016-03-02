use hyper::client::{Client,Pool};
use hyper::net::HttpsConnector;
use hyper::net::Openssl;
use model::{PrePayResult,de_xml};
use std::io::Read;
use std::default::Default;
use std::sync::Arc;

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
    attach : Option<String>,
    out_trade_no : String,
    fee_type : Option<String>,
    total_fee : i32,
    spbill_create_ip : String,
    time_start : Option<String>,
    time_expire : Option<String>,
    goods_tag : Option<String>,
    notify_url : String,
    trade_type : String,
    product_id : Option<String>,
    limit_pay : Option<String>,
    openid : Option<String>
}

impl PrePay {

	pub fn new() -> PrePay {
		PrePay{appid : "String".to_string(),
		    mch_id : "String".to_string(),
		    device_info : None,
		    nonce_str : "String".to_string(),
		    sign : "String".to_string(),
		    body : "String".to_string(),
		    detail : None,
		    attach : None,
		    out_trade_no : "String".to_string(),
		    fee_type : None,
		    total_fee : 0,
		    spbill_create_ip : "String".to_string(),
		    time_start : None,
		    time_expire : None,
		    goods_tag : None,
		    notify_url : "String".to_string(),
		    trade_type : "String".to_string(),
		    product_id : None,
		    limit_pay : None,
		    openid : None
		}
	}

    fn to_xml(&self) -> String {
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
					   <spbill_create_ip>{spbill_create_ip}</spbill_create_ip>
					   <total_fee>{total_fee}</total_fee>
					   <trade_type>JSAPI</trade_type>
					   <sign>{sign}</sign>
					</xml>
		    	"#,appid="testappid",attach="attach",body="body",mch_id="mch_id",nonce_str="nonce_str",
		    	notify_url="notify_url",openid="openid",out_trade_no="out_trade_no",spbill_create_ip="spbill_create_ip",
		    	total_fee="1000",sign="sign")
    }
}

pub fn ssl_client() -> Client {

	let mut ctx = SslContext::new(SslMethod::Sslv23).unwrap();
    ctx.set_cipher_list("DEFAULT").unwrap();
    //try!(ctx.set_certificate_file(cert.as_ref(), X509FileType::PEM));
    //try!(ctx.set_private_key_file(key.as_ref(), X509FileType::PEM));
    ctx.set_verify(SSL_VERIFY_NONE, None);
	let https = HttpsConnector::new(Openssl { context: Arc::new(ctx) });
	let pool = Pool::with_connector(Default::default(),https);
	return Client::with_connector(pool);
}

pub fn pre_pay(p : PrePay) -> Result<PrePayResult,&'static str> {
	//let client = Client::new();
	let client = ssl_client();
	if let Ok(ref mut res) = client.post("https://api.mch.weixin.qq.com/pay/unifiedorder").body(&p.to_xml()).send() {
		let mut buf = String::new();
		if let Ok(_) = res.read_to_string(& mut buf) {
			println!("{}", buf);
			if let Ok(prepay_resutl) = de_xml::<PrePayResult>(&buf) {
				return Ok(prepay_resutl);
			}
		}
	}
	Err("prepay error!!!")  
}



