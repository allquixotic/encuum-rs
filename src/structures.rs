use serde::{Serialize, Deserialize};

macro_rules! pub_struct {
    ($name:ident {$($field:ident: $t:ty,)*}) => {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
        pub struct $name {
            $(pub $field: $t),*
        }
    };
}

pub_struct!(ScrapeOpts {
    headless: bool,
    baseurl: String,
    username: String,
    password: String,
    forumbase: String,
});

pub_struct!(BaseEntity {
    url: String,
    title: String,
});

pub_struct!(Post {
    url: String,
    poster_name: String,
    bbcode: String,
    post_sequence: u64,
});

pub_struct!(ForumThread {
    base: BaseEntity,
    poster_name: String,
    replies: Vec<Post>,
});

pub_struct!(Forum {
    base: BaseEntity,
    threads: Vec<ForumThread>,
});
