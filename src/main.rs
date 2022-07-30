fn main() {
    let users = query_user();

    dbg!(users);
}

fn query_user() -> Vec<User> {
    use regex::Regex;
    use std::process::Command;

    if !cfg!(target_os = "windows") {
        panic!("supported only windows")
    }

    let out = Command::new("cmd")
        .args(vec!["/c", "chcp 65001 && query user"])
        .output()
        .expect("command error");
    let text = String::from_utf8(out.stdout).expect("not utf8 string");
    println!("{}", text);

    let lf_text = text.replace("\r", "");
    // convert double over white space to tab
    let re = Regex::new(r"  +").expect("regex format err");
    let tsv_text = re.replace_all(&lf_text, "\t");
    let lines = tsv_text.split("\n");

    let users = lines
        // skip `active code page` message & headers
        .skip(2)
        .map(|line| {
            let items: Vec<_> = line.split("\t").collect();
            dbg!(line);
            if items.len() > 4 {
                Some(User {
                    user_name: items.get(0).unwrap().to_string(),
                    session_name: items.get(1).unwrap().to_string(),
                    id: items.get(2).unwrap().to_string(),
                    state: items.get(3).unwrap().to_string(),
                    idle_time: items.get(4).unwrap().to_string(),
                    login_time: items.get(5).unwrap().to_string(),
                })
            } else {
                None
            }
        })
        .flatten()
        .collect();

    return users;
}

#[derive(Debug)]
#[allow(dead_code)]
struct User {
    user_name: String,
    session_name: String,
    id: String,
    state: String,
    idle_time: String,
    login_time: String,
}
