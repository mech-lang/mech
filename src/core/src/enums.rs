use hashbrown::HashSet;

#[derive(Debug,Clone,PartialEq,PartialOrd,Serialize,Deserialize)]
pub struct Enum {
    pub id: u64,
    variants: Vec<u64>,
}