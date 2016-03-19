use toml;
use toml::{Table};
use std::fs::File;
use std::io::{Read};


lazy_static! {
    static ref global_config: Table = get_config();
}


// 这个的好处在于可以实时更新，不需要重启服务
fn get_config () -> Table {
	let mut input = String::new();
	File::open("./config.toml").and_then(|mut f| {
		f.read_to_string(&mut input)
	}).unwrap();
	toml::Parser::new(&input).parse().unwrap()
}


pub struct ConfigManager;

impl ConfigManager {

	pub fn get_config_num<'a> (section: &'a str, attr_name: &'a str) -> i64 {
		let config = &global_config;
		let sec = config.get(section).unwrap();
		
		let num = sec.lookup(&attr_name).unwrap().as_integer().unwrap();
		
		num as i64
	}

	pub fn get_config_str<'a> (section: &'a str, attr_name: &'a str) -> String {
		let config = &global_config;
		let sec = config.get(section).unwrap();
		
		let astr = sec.lookup(&attr_name).unwrap().as_str().unwrap();
		
		astr.to_owned()
	}

	pub fn get_config_num_arr<'a> (section: &'a str, attr_name: &'a str) -> Vec<i64> {
		let config = &global_config;
		let sec = config.get(section).unwrap();
		
		let arr = sec.lookup(&attr_name).unwrap().as_slice().unwrap();
		// convert to config number to i64
		let mut vecret: Vec<i64> = Vec::new();
		for num in arr {
			//println!("{}", room);
			vecret.push(num.as_integer().unwrap());
		}
		
		vecret
	}


	pub fn get_config_str_arr<'a> (section: &'a str, attr_name: &'a str) -> Vec<String> {
		let config = &global_config;
		let sec = config.get(section).unwrap();
		
		let arr = sec.lookup(&attr_name).unwrap().as_slice().unwrap();
		// convert to config number to i64
		let mut vecret: Vec<String> = Vec::new();
		for astr in arr {
			//println!("{}", room);
			vecret.push(astr.as_str().unwrap().to_owned());
		}
		
		vecret
	}

}




