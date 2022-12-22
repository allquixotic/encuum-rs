mod scraping;
mod structures;
mod writer;
use dotenv;
use scraping::scrape;
use structures::ScrapeOpts;

fn get_env_bool(name: &str, def: bool) -> bool {
    match dotenv::var(name) {
        Ok(val) => match val.to_lowercase().parse() {
            Ok(b) => b,
            Err(_) => def
        },
        Err(_) => def,
    }
}

fn get_req_env(name: &str) -> String {
    dotenv::var(name).expect(&format!("Required environment variable missing: {}. Exiting. \
    You can either export it into the environment, or create a .env file (see dotenv crate).", name))
}

fn main() {
    let opts = ScrapeOpts {
        headless: get_env_bool("headless", true),
        baseurl: get_req_env("baseurl"),
        username: get_req_env("username"),
        password: get_req_env("password"),
        forumbase: get_req_env("forumbase"),
    };
    let rslt = scrape(opts);
    match rslt {
        Ok(forums) => writer::write_forums_to_files(forums),
        Err(e) => println!("ERROR: Top-level error means something unhandleable happened during web scraping: {}", e.as_ref())
    }
    
}
