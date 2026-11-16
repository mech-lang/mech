use serde::{Deserialize, Serialize};

pub const NUM_TOP_ITEMS: usize = 5;

#[derive(Serialize, Deserialize)]
#[repr(align(64))]
pub struct Post<'a> {
    #[serde(rename = "_id")]
    pub id: &'a str,
    pub title: &'a str,
    pub tags: Vec<&'a str>,
}

#[derive(Serialize)]
#[repr(align(64))]
pub struct RelatedPosts<'a> {
    #[serde(rename = "_id")]
    pub id: &'a str,
    pub tags: &'a [&'a str],
    pub related: [&'a Post<'a>; NUM_TOP_ITEMS],
}
