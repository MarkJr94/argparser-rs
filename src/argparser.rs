//! This module defines and contains all the important
//! argument parsing functionality. The requisite types
//! and functions are re-exported at the top-level of
//! the crate.

use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash};
use std::str::FromStr;

use slide::{Slider};

/// This enum represents the different types of arguments supported
#[derive(Debug, Clone, PartialEq)]
pub enum ArgType {
    /// An argument that takes a value, as in `./go --pic lol.jpg`
    Option,
    /// An argument that is a simple flag, as in `rustc --version`
    Flag,
    /// Like an `Option`, but takes multiple values, as in 
    /// `./go --pics 1.png 2.png 3.png`
    List,
    /// Like a `List` but takes colon-split key-value pairs, as in
    /// `./go --pics Monday:1.jpg Tuesday:2.jpg`
    Dict,
    /// A positional argument, as in `rustc lib.rs`. The u8 indicates
    /// The relative position of the position argument (i.e. `Positional(0)`
    /// indicates that this is the first positional argument
    Positional(u8),
}

impl ArgType {
    fn is_positional(&self) -> bool {
        match self {
            &ArgType::Positional(_) => true,
            _ => false,
        }
    }
}

impl fmt::Display for ArgType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match self {
            &ArgType::Option => "Option",
            &ArgType::Flag => "Flag",
            &ArgType::List => "List",
            &ArgType::Dict => "Dict",
            &ArgType::Positional(_) => "Positional"
        };
        
        write!(f, "{}", msg)
    }
}

#[derive(Debug, Clone)]
struct Arg {
    val: Option<String>,
    count: u16,
    required: bool,
    flag: char,
    help: String,
    type_: ArgType,
}

#[derive(Debug, Clone)]
/// This type represents the state and methods for parsing arguments.
/// A new parser must be created for every set of arguments you want to parse.
pub struct ArgParser {
    arguments: HashMap<String, Arg>,
    name: String,
    done: bool,
}

/// Simple type alias to reduce typing. The return type of
/// `ArgParser::parse`.
pub type ParseResult = Result<ArgParseResults, String>;

impl ArgParser {
    /// Constructs a new `ArgParser`, given the name of the program
    /// that you want to be printed in help messages
    pub fn new(name: String) -> ArgParser {
        let mut me = ArgParser {
            arguments: HashMap::new(),
            name: name,
            done: false,
        };

        me.add_opt("help", Some("false"), 'h', false, 
            "Show this help message", ArgType::Flag);
        
        me
    }
    
    /// Add another option to parse.
    /// # Example
    /// ```
    /// // add an option that is a `Flag`, with no default value, with
    /// // a long form of `--verbose`, short form of `v`, that is not
    /// // required to be passed, and has a default value of `false`
    ///
    /// use argparse::{ArgParser, ArgType};
    ///
    /// let mut parser = ArgParser::new("runner".into());
    /// parser.add_opt("verbose", Some("false"), 'v', false,
    ///     "Whether to produce verbose output", ArgType::Flag);
    /// ```
    pub fn add_opt(&mut self, name: &str, 
        default: Option<&str>, flag: char, required: bool, 
        help: &str, type_: ArgType) {
        
        let o = Arg {
            val: default.map(|x| x.into()), 
            count: 0, 
            required: required,
            flag: flag,
            help: help.into(),
            type_: type_,
        };
        
        self.arguments.insert(name.into(), o);
    }
    
    /// Remove an option from parsing consideration.
    /// # Example
    /// ```
    /// // add an option that is a `Flag`, with no default value, with
    /// // a long form of `--verbose`, short form of `v`, that is not
    /// // required to be passed, and has a default value of `false`
    ///
    /// use argparse::{ArgParser, ArgType};
    ///
    /// let mut parser = ArgParser::new("runner".into());
    /// parser.add_opt("verbose", Some("false"), 'v', false,
    ///     "Whether to produce verbose output", ArgType::Flag);
    /// assert!(parser.remove_opt("verbose").is_ok())
    /// ```
    pub fn remove_opt(&mut self, name: &str) -> Result<(), &'static str> {
        
        self.arguments.remove(name).map(|_| ()).ok_or("No such Option")
    }
    
    /// Parse a set of arguments, given the previous configuration
    /// # Example
    /// ```
    /// // add an option that is a `Flag`, with no default value, with
    /// // a long form of `--verbose`, short form of `v`, that is not
    /// // required to be passed, and has a default value of `false`
    ///
    /// use argparse::{ArgParser, ArgType};
    ///
    /// let mut parser = ArgParser::new("runner".into());
    /// parser.add_opt("verbose", Some("false"), 'v', false,
    ///     "Whether to produce verbose output", ArgType::Flag);
    ///
    /// // Normally you'd get this from std::env::args().iter()
    /// let test_1 = "./runner --verbose".split_whitespace()
    ///     .map(|s| s.into())
    ///     .collect::<Vec<String>>();
    /// 
    /// if let Ok(p_res) = parser.parse(test_1.iter()) {
    ///     // do stuff here
    /// }
    /// ```
    pub fn parse<'a, I: Iterator<Item = &'a String>> (&self, args: I) -> ParseResult {
        use std::collections::hash_map::Entry;
        
        if self.arguments.len() == 0 || self.done {
            return Err("No arguments given to parse".into());
        }
        
        let argvec: Vec<String> = separate_flags(args.map(|s| s.clone()).collect());
        
        let mut taken_up = Vec::new();
        let mut new_args = self.arguments.clone();
        
        for (argname, my_arg) in self.arguments.iter() {
            for (flag, rest) in argvec.slide().filter(|&(f, _)| {f == &format!("-{}", my_arg.flag) || f == &format!("--{}", argname)}) {

                if let Entry::Occupied(mut e) = new_args.entry(argname.clone()) {
                    let arg = e.get_mut();
                    arg.count = arg.count + 1;
                    taken_up.push(flag);
                    
                    match arg.type_ {
                        ArgType::Flag => { arg.val = Some("true".into()); }
                        ArgType::Option => {
                            let err = format!("This option `{}` requires a value you have not provided", argname);
                            
                            if let Some(rest) = rest {
                                if is_flag(&rest[0]) || is_long_flag(&rest[0]) {
                                    return Err(err);
                                }
                                
                                arg.val = Some(rest[0].clone());
                                taken_up.push(&rest[0]);
                            } else {
                                return Err(err);
                            }
                        }
                        ArgType::List | ArgType::Dict => {
                            if let Some(rest) = rest {
                                arg.val = Some(rest.iter()
                                    .take_while(|x| !(is_flag(x) || is_long_flag(x)))
                                    .fold(String::new(), |mut acc, elem| {
                                        acc.push_str(elem);
                                        acc.push(' ');
                                        acc
                                    }));
                                    
                                taken_up.extend(rest.iter().take_while(|x| !(is_flag(x) || is_long_flag(x))));
                            } else {
                                let err = format!("This option `{}` requires a value you have not provided", argname);
                                return Err(err);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        
        for (_, ref mut v) in new_args.iter_mut().filter(|&(_, ref vv)| vv.val.is_none() && vv.type_.is_positional()) {
            
            if let Some((_, x)) = argvec.iter().skip(1)
                .filter(|e| !taken_up.contains(e))
                .enumerate()
                .find(|&(i, _)| {
                    if let ArgType::Positional(idx) = v.type_ {
                        idx as usize == i
                    } else {
                        false
                    }
                }) {
                
                    v.val = Some(x.clone());
            }
        }

        if !new_args.iter().all(|(_, v)| !v.required | v.val.is_some()) {
            return Err("Not all required arguments are found".into());
        }
        
        let res = ArgParseResults::new(self.name.clone(), new_args);
        res.p_args();
        
        Ok(res)
    }

    /// Prints the help message, which is constructed based on the options
    /// used
    /// # Example
    /// ```
    /// use argparse::{ArgParser, ArgType};
    ///
    /// let mut parser = ArgParser::new("runner".into());
    /// parser.add_opt("verbose", Some("false"), 'v', false,
    ///     "Whether to produce verbose output", ArgType::Flag);
    ///
    /// // Normally you'd get this from std::env::args().iter()
    /// let test_1 = "./runner --help".split_whitespace()
    ///     .map(|s| s.into())
    ///     .collect::<Vec<String>>();
    /// 
    /// if let Ok(p_res) = parser.parse(test_1.iter()) {
    ///     if let Some(true) = p_res.get("help") {
    ///         parser.help();
    ///     }
    /// }
    /// ```
    pub fn help(&self) {
        print!("Usage:\t./{} ", self.name);
        
        for (argname, info) in self.arguments.iter() {
            print!("[--{} {}] ", argname, ops(info, argname));
        }
        println!("");
        
        print!("Options:\n\n");
        for (argname, info) in self.arguments.iter() {            
            print!("--{} (-{})\t", argname, info.flag);
            print!("Required: {}\t", info.required);
            print!("Type: {}\n", info.type_);
            print!("\t");
            
            let mut i = 0;
            for c in info.help.chars() {
                print!("{}", c);
                
                if i > 60 && c.is_whitespace() {
                    print!("\n\t\t");
                    i = 0;
                }
                
                i = i + 1;
            }
            
            println!("\n");
        }
    }
}

#[derive(Debug, Clone)]
/// This type represents the result ofparsing arguments.
pub struct ArgParseResults {
    arguments: HashMap<String, Arg>,
    name: String,
}

impl ArgParseResults {

    fn new(name: String, args: HashMap<String, Arg>) -> ArgParseResults {
        ArgParseResults { name: name, arguments: args }
    }

    #[inline]
    #[cfg(debug_assertions)]
    fn p_args(&self) {
        for (k, v) in self.arguments.iter() {
            println!("{}:{:?}", k, v.val);
        }
    }
    
    #[inline]
    #[cfg(not(debug_assertions))]
    fn p_args(&self) {}
    
    /// Extracts the argument, as long is the value type implements
    /// `FromStr`
    /// # Example
    /// ```
    /// use argparse::{ArgParser, ArgType};
    ///
    /// let mut parser = ArgParser::new("runner".into());
    /// parser.add_opt("verbose", Some("false"), 'v', false,
    ///     "Whether to produce verbose output", ArgType::Flag);
    ///
    /// // Normally you'd get this from std::env::args().iter()
    /// let test_1 = "./runner -v".split_whitespace()
    ///     .map(|s| s.into())
    ///     .collect::<Vec<String>>();
    /// 
    /// if let Ok(p_res) = parser.parse(test_1.iter()) {
    ///     if let Some(true) = p_res.get::<bool>("verbose") {
    ///         // be verbose
    ///     }
    /// }
    /// ```
    pub fn get<T: FromStr>(&self, name: &str) -> Option<T> {
        if let Some(ref arg) = self.arguments.get(name.into()) {
            arg.val.as_ref().and_then(|x| x.parse().ok())
        } else {
            None
        }
    }
    
    /// Extracts the argument, using the `ArgGetter<T>` that you provided
    ///
    /// # Note
    /// See documentation for the trait [`ArgGetter`](./trait.ArgGetter.html) for more information
    /// 
    /// # Example
    /// ```
    /// use argparse::{ArgParser, ArgType};
    ///
    /// let mut parser = ArgParser::new("runner".into());
    /// parser.add_opt("verbose", Some("false"), 'v', false,
    ///     "Whether to produce verbose output", ArgType::Flag);
    ///
    /// // Normally you'd get this from std::env::args().iter()
    /// let test_1 = "./runner -v".split_whitespace()
    ///     .map(|s| s.into())
    ///     .collect::<Vec<String>>();
    /// 
    /// let dumb_closure = |_: &str| { Some(true) };
    /// 
    /// if let Ok(p_res) = parser.parse(test_1.iter()) {
    ///     if let Some(true) = p_res.get_with::<bool, _>("verbose", dumb_closure) {
    ///         // be verbose
    ///     }
    /// }
    /// ```
    pub fn get_with<T, P>(&self, name: &str, parser: P) -> Option<T>
    where P: ArgGetter<T> {
        if let Some(ref arg) = self.arguments.get(name.into()) {
            arg.val.as_ref().and_then(|x| parser.get_arg(&x))
        } else {
            None
        }
    }
}

/// Represents something capable of turning a `&str` in the value
/// type of your choice. Implement this to use with `ArgParseResults::get_with`
///
/// # Note
/// An implementation is provided for all closures of type `F: FnOnce(&str) -> Option<T>`
pub trait ArgGetter<T> {
    /// This is the key function that converts from a string 
    /// to the required value tpe
    fn get_arg(self, s: &str) -> Option<T>;
}

impl<T, F: FnOnce(&str) -> Option<T>> ArgGetter<T> for F {
    fn get_arg(self, s: &str) -> Option<T> {
        self(s)
    }
}

/// Function that parses `List` arguments into `Vec`s.
/// Provided for user convenience and use as an implementor of
/// [`ArgGetter`](./trait.ArgGetter.html).
pub fn vec_parser<T: FromStr>(s: &str) -> Option<Vec<T>> {
    s.split_whitespace()
        .map(|x| x.parse())
        .enumerate()
        .fold(None, |acc, (idx, elem)| {
            if let Ok(x) = elem {
                if idx == 0 {
                    return Some(vec![x]);
                } else {
                    return acc.map(|mut v| {
                        v.push(x);
                        v
                    });
                }
            } else {
                return None;
            }
        })
}

/// Function that parses `Dict` arguments into `HashMap`s.
/// Provided for user convenience and use as an implementor of
/// [`ArgGetter`](./trait.ArgGetter.html).
/// # Panics
/// Panics if improper or no separator is found (expects `key:value key2:value2...`)
pub fn hashmap_parser<K, V>(s: &str) -> Option<HashMap<K,V>> 
    where K: FromStr + Hash + Eq,
          V: FromStr {
    s.split_whitespace()
        .map(|x| {
            let colpos = x.find(':')
                .expect("No separator found in dict map argument");
            let (k, v) = x.split_at(colpos);
            let v = &v[1..];
            (k, v)
        })
        .map(|(k, v)| {
            k.parse().ok().and_then(|k2|
                v.parse().ok().map(|v2| (k2, v2)))
        })
        .enumerate()
        .fold(None, |acc, (idx, elem)| {
            if let Some((k, v)) = elem {
                if idx == 0 {
                    let mut h = HashMap::new();
                    h.insert(k,v);
                    return Some(h);
                } else {
                    return acc.map(|mut h| {
                        h.insert(k, v);
                        h
                    });
                }
            } else {
                return None;
            }
        })
}

fn ops(a: &Arg, name: &str) -> String {
    if a.type_ == ArgType::Option {
        name.chars().map(|c| c.to_uppercase().next().unwrap_or(c)).collect::<String>()
    } else if a.type_ == ArgType::List {
        name.chars().map(|c| c.to_uppercase().next().unwrap_or(c)).chain("...".chars()).collect::<String>()
    } else if a.type_ == ArgType::Dict {
        "k:v k2:v2...".into()
    } else {
        String::new()
    }
}

fn is_flag(s: &str) -> bool {
    if s.len() < 2 {
        return false;
    }
    
    let v: Vec<char> = s.chars().collect();
    
    if v[0] == '-' {
        if v[1].is_alphabetic() {
            return true;
        }
    }
    
    false
}

fn is_long_flag(s: &str) -> bool {
    if s.len() < 3 {
        return false;
    }
    
    let v: Vec<char> = s.chars().collect();
    
    if v[0] == v[1] && v[1] == '-' {
        return true;
    }
    
    false
}

fn separate_flags(og: Vec<String>) -> Vec<String> {
    let mut separated = Vec::new();
    
    for x in og {
        if is_long_flag(&x) {
            separated.push(x);
        } else if is_flag(&x) {
            if x.len() == 2 {
                separated.push(x);
            } else {
                for short_flag in x.chars().skip(1) {
                    separated.push(format!("-{}", short_flag));
                }
            }
        } else {
            separated.push(x);
        }
    }
    
    return separated;
}

#[cfg(test)]
mod test {
    use super::{ArgParser, ArgType, vec_parser, hashmap_parser};
    use std::collections::HashMap;
    const LONG_STR: &'static str = r#"Check your proxy settings or contact your network administrator to make sure the proxy server is working. If you don't believe you should be using a proxy server: Go to the Chromium menu > Settings > Show advanced settings... > Change proxy settings... and make sure your configuration is set to "no proxy" or "direct.""#;
    
    fn setup_1() -> ArgParser {
        let mut parser = ArgParser::new("ArgParsers".into());
        
        parser.add_opt("length", None, 'l', true, LONG_STR, ArgType::Option);
        parser.add_opt("height", None, 'h', true, "Height of user in centimeters", ArgType::Option);
        parser.add_opt("name", None, 'n', true, "Name of user", ArgType::Option);
        parser.add_opt("frequencies", None, 'f', false, "User's favorite frequencies", ArgType::List);
        parser.add_opt("mao", Some("false"), 'm', false, "Is the User Chairman Mao?", ArgType::Flag);
        
        parser
    }
    
    #[test]
    fn test_parser() {
        let parser = setup_1();
    
        let test_1 = "./go -l -60 -h -6001.45e-2 -n Johnny --mao -f 1 2 3 4 5".split_whitespace()
            .map(|s| s.into())
            .collect::<Vec<String>>();
        
        let p_res = parser.parse(test_1.iter()).unwrap();
        
        assert!(p_res.get("length") == Some(-60));
        assert_eq!(p_res.get("height"), Some(-6001.45e-2));
        assert_eq!(p_res.get::<String>("name"), Some("Johnny".into()));
        assert_eq!(p_res.get_with("frequencies", vec_parser), 
            Some(vec![1,2,3,4,5]));
        assert_eq!(p_res.get("mao"), Some(true));
        
        parser.help();
    }
    
    #[test]
    fn test_parser_unrequired() {
        let parser = setup_1();
        
        let test_1 = "./go -l -60 -h -6001.45e-2 -n Johnny -f 1 2 3 4 5".split_whitespace()
            .map(|s| s.into())
            .collect::<Vec<String>>();
            
        let p_res = parser.parse(test_1.iter()).unwrap();
        
        assert!(p_res.get("length") == Some(-60));
        assert_eq!(p_res.get("height"), Some(-6001.45e-2));
        assert_eq!(p_res.get::<String>("name"), Some("Johnny".into()));
        assert_eq!(p_res.get_with("frequencies", vec_parser), 
            Some(vec![1,2,3,4,5]));
        assert_eq!(p_res.get("mao"), Some(false));
        
        parser.help();
    }
    
    #[test]
    fn test_parser_unrequired_nodefault() {
        let parser = setup_1();
        
        let test_1 = "./go -l -60 -h -6001.45e-2 -n Johnny".split_whitespace()
            .map(|s| s.into())
            .collect::<Vec<String>>();
            
        let p_res = parser.parse(test_1.iter()).unwrap();
        
        assert!(p_res.get("length") == Some(-60));
        assert_eq!(p_res.get("height"), Some(-6001.45e-2));
        assert_eq!(p_res.get::<String>("name"), Some("Johnny".into()));
        assert_eq!(p_res.get_with::<Vec<u8>, _>("frequencies", vec_parser), None);
        assert_eq!(p_res.get("mao"), Some(false));
        
        parser.help();
    }
    
    #[test]
    fn test_parser_dict() {
        let mut parser = setup_1();
        parser.add_opt("socks", None, 's', false, "If you wear socks that day", ArgType::Dict);
        
        let test_1 = "./go -l -60 -h -6001.45e-2 -n Johnny -s Monday:true Friday:false".split_whitespace()
            .map(|s| s.into())
            .collect::<Vec<String>>();
            
        let p_res = parser.parse(test_1.iter()).unwrap();
        
        assert!(p_res.get("length") == Some(-60));
        assert_eq!(p_res.get("height"), Some(-6001.45e-2));
        assert_eq!(p_res.get::<String>("name"), Some("Johnny".into()));
        assert_eq!(p_res.get_with::<Vec<u8>, _>("frequencies", vec_parser), None);
        assert_eq!(p_res.get("mao"), Some(false));
        
        let h = [("Monday", true), ("Friday", false)]
            .iter()
            .map(|&(k, v)| (k.into(), v))
            .collect();
            
        assert_eq!(p_res.get_with::<HashMap<String, bool>, _>("socks", hashmap_parser),
            Some(h));
        
        parser.help();
    }
    
    #[test]
    fn test_parser_positional() {
        let mut parser = setup_1();
        
        parser.add_opt("csv", None, 'c', true, "csv input file",
            ArgType::Positional(0));
        parser.add_opt("json", None, 'j', true, "json output file",
            ArgType::Positional(1));
        
        let test_1 = "./go -l -60 -h -6001.45e-2 -n Johnny crap.csv crap.json".split_whitespace()
            .map(|s| s.into())
            .collect::<Vec<String>>();
            
        let p_res = parser.parse(test_1.iter()).unwrap();
        
        assert!(p_res.get("length") == Some(-60));
        assert_eq!(p_res.get("height"), Some(-6001.45e-2));
        assert_eq!(p_res.get::<String>("name"), Some("Johnny".into()));
        assert_eq!(p_res.get_with::<Vec<u8>, _>("frequencies", vec_parser), None);
        assert_eq!(p_res.get("mao"), Some(false));
        assert_eq!(p_res.get::<String>("csv"), Some("crap.csv".into()));
        assert_eq!(p_res.get::<String>("json"), Some("crap.json".into()));
        
        parser.help();
    }
}