use hashbrown::HashMap;

use crate::types::Post;

pub fn get_post_tags_map<'a>(posts: &'a [Post]) -> HashMap<&'a str, Vec<u32>> {
    let mut post_tags_map: HashMap<&str, Vec<u32>> = HashMap::with_capacity(128);

    for (i, post) in posts.iter().enumerate() {
        for tag in &post.tags {
            post_tags_map
                .entry(tag)
                .or_insert_with(|| Vec::with_capacity(1024))
                .push(i as u32);
        }
    }

    post_tags_map
}
