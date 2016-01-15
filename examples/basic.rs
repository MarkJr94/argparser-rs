extern crate argparse;

use std::collections::HashMap;

use argparse::{ArgParser, ArgType, hashmap_parser, vec_parser};
const LONG_STR: &'static str = r#"Check your proxy settings or contact your network administrator to make sure the proxy server is working. If you don't believe you should be using a proxy server: Go to the Chromium menu > Settings > Show advanced settings... > Change proxy settings... and make sure your configuration is set to "no proxy" or "direct.""#;

fn main() {
    let mut parser = ArgParser::new("argparse".into());
    
    parser.add_opt("length", None, 'l', true,
        LONG_STR, ArgType::Option);
    parser.add_opt("height", None, 'h', true,
        "Height of user in centimeters", ArgType::Option);
    parser.add_opt("name", None, 'n', true,
        "Name of user", ArgType::Option);
    parser.add_opt("frequencies", None, 'f', false,
        "User's favorite frequencies", ArgType::List);
    parser.add_opt("mao", Some("false"), 'm', false,
        "Is the User Chairman Mao?", ArgType::Flag);
    parser.add_opt("socks", None, 's', false,
        "If you wear socks that day", ArgType::Dict);
    
    let test_1 = "./go -l -60 -h -6001.45e-2 -n Johnny -m -f 1 2 3 4 5 -s Monday:true Friday:false".split_whitespace()
        .map(|s| s.into())
        .collect::<Vec<String>>();

    let p_res = parser.parse(test_1.iter()).unwrap();

    let str_to_veci32 = |s: &str| {
        Some(s.split_whitespace().map(|s| s.parse().unwrap())
            .collect::<Vec<i32>>())
    };
    
    assert!(p_res.get("length") == Some(-60));
    assert_eq!(p_res.get("height"), Some(-6001.45e-2));
    assert_eq!(p_res.get::<String>("name"), Some("Johnny".into()));
    assert_eq!(p_res.get_with("frequencies", str_to_veci32), 
        Some(vec![1,2,3,4,5]));
    assert_eq!(p_res.get_with("frequencies", vec_parser), 
        Some(vec![1,2,3,4,5]));
    assert_eq!(p_res.get("mao"), Some(true));
    
    let h = [("Monday", true), ("Friday", false)]
        .iter()
        .map(|&(k, v)| (k.into(), v))
        .collect();
        
    assert_eq!(p_res.get_with::<HashMap<String, bool>, _>("socks", hashmap_parser),
        Some(h));

    parser.help();
}