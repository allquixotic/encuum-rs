use std::fs::File;
use std::io::Write;
use crate::structures::Forum;
use serde_json;

pub fn write_forums_to_files(forums: Vec<Forum>) {
    for f in forums {
        write_forum(f);
    }
}

pub fn write_forum(forum: Forum) {
    let filename = forum.base.title.replace(r"[^A-Za-z0-9 ]", "");
    let fo = File::create(&filename);
    if fo.is_err() {
        println!("ERROR opening file {}", &filename);
        return;
    }
    let serialized = serde_json::to_string(&forum);
    if serialized.is_err() {
        println!("ERROR serializing forum {}", forum.base.title);
        return;
    }
    if fo.unwrap().write_all(serialized.unwrap().as_bytes()).is_err() {
        println!("ERROR writing file {}", filename);
    };
}