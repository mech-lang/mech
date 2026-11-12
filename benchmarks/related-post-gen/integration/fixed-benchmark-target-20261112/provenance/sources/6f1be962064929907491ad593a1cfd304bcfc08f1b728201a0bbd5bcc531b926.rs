//! Diagnostic native control for the optimized Mech numeric algorithm.
//! Uses checked release arithmetic and bounds-checked u64 vectors and byte-bounded U32 rank arithmetic; Mech's
//! finite-resource accounting and immutable snapshot machinery are absent.
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::{fs, io::{self, BufWriter}, time::Instant};

#[derive(Deserialize, Serialize)]
struct Post<'a> { #[serde(rename="_id")] id: &'a str, title: &'a str, tags: Vec<&'a str> }
#[derive(Serialize)]
struct Related<'a> { #[serde(rename="_id")] id: &'a str, tags: &'a [&'a str], related: [&'a Post<'a>; 5] }
const LOW: u64 = (1u64 << 58) - 1;
fn rank(score: u64, post: u64) -> u64 { score.wrapping_shl(58) | (LOW ^ post) }
fn insert(t: [u64;5], r: u64) -> [u64;5] {
    if r>t[0] {[r,t[0],t[1],t[2],t[3]]}
    else if r>t[1] {[t[0],r,t[1],t[2],t[3]]}
    else if r>t[2] {[t[0],t[1],r,t[2],t[3]]}
    else if r>t[3] {[t[0],t[1],t[2],r,t[3]]}
    else {[t[0],t[1],t[2],t[3],r]}
}
fn key3(a:u64,b:u64,c:u64)->u64 {
    let a=u32::from(u8::try_from(a).expect("eligible tag ID fits a byte"));
    let b=u32::from(u8::try_from(b).expect("eligible tag ID fits a byte"));
    let c=u32::from(u8::try_from(c).expect("eligible tag ID fits a byte"));
    let low=a.min(b).min(c);let high=a.max(b).max(c);let middle=a+b+c-low-high;
    u64::from(low+middle*(middle-1)/2+high*(high-1)*(high-2)/6)
}
fn key4(a:u64,b:u64,c:u64,d:u64)->u64 {
    let a=u32::from(u8::try_from(a).expect("eligible tag ID fits a byte"));
    let b=u32::from(u8::try_from(b).expect("eligible tag ID fits a byte"));
    let c=u32::from(u8::try_from(c).expect("eligible tag ID fits a byte"));
    let d=u32::from(u8::try_from(d).expect("eligible tag ID fits a byte"));
    let al=a.min(b);let ah=a.max(b);let cl=c.min(d);let ch=c.max(d);
    let low=al.min(cl);let high=ah.max(ch);let ma=al.max(cl);let mb=ah.min(ch);
    let second=ma.min(mb);let third=ma.max(mb);
    u64::from(low+second*(second-1)/2+third*(third-1)*(third-2)/6+high*(high-1)*(high-2)*(high-3)/24)
}
fn subsets(offsets:&[u64],ids:&[u64],width:usize)->(Vec<u64>,Vec<u64>) {
    let mut boundary=Vec::with_capacity(offsets.len());boundary.push(0);
    let mut keys=Vec::with_capacity(ids.len()*if width==3{7}else{14});
    for post in 0..offsets.len()-1 {
        let first=offsets[post] as usize;let end=offsets[post+1] as usize;
        for a in first..end {for b in a+1..end {for c in b+1..end {
            if width==3 {keys.push(key3(ids[a],ids[b],ids[c]));}
            else {for d in c+1..end {keys.push(key4(ids[a],ids[b],ids[c],ids[d]));}}
        }}}
        boundary.push(keys.len() as u64);
    }
    (boundary,keys)
}
fn index(offsets:&[u64],keys:&[u64],buckets:usize)->(Vec<u64>,Vec<u64>,Vec<u64>) {
    let mut counts=vec![0u64;buckets];for &key in keys {counts[key as usize]+=1;}
    let mut boundary=Vec::with_capacity(buckets+1);boundary.push(0u64);
    for &count in &counts {boundary.push(boundary[boundary.len()-1]+count);}
    let mut positions=boundary.clone();let mut postings=vec![0u64;keys.len()];
    for post in 0..offsets.len()-1 {for at in offsets[post]..offsets[post+1] {
        let key=keys[at as usize] as usize;let destination=positions[key];
        postings[destination as usize]=post as u64;positions[key]=destination+1;
    }}
    (counts,boundary,postings)
}
fn ordinary(offsets:&[u64],ids:&[u64],vocabulary:usize)->Vec<u64> {
    let n=offsets.len()-1;let (_,boundaries,postings)=index(offsets,ids,vocabulary);
    let mut counts=vec![0u8;n.next_multiple_of(64)];let mut out=Vec::with_capacity(n*5);
    for query in 0..n {
        for at in offsets[query]..offsets[query+1] {let key=ids[at as usize] as usize;
            for cursor in boundaries[key]..boundaries[key+1] {let p=postings[cursor as usize] as usize;counts[p]=counts[p].wrapping_add(1);}
        }
        counts[query]=0;let mut top=[(0u8,0u64);5];
        for (post,&score)in counts.iter().enumerate(){if score>top[4].0 {
            top[4]=(score,post as u64);for i in (0..4).rev(){if top[i].0>=score{break;}top.swap(i,i+1);}
        }}
        out.extend(top.map(|(_,p)|p));counts.fill(0);
    }
    out
}
fn streaming_histogram<'a>(tags:&[&'a str])->Vec<(&'a str,u64)> {
    let mut counts=HashMap::new();
    for &tag in tags {*counts.entry(tag).or_insert(0u64)+=1;}
    let mut histogram:Vec<_>=counts.into_iter().collect();
    histogram.sort_unstable_by(|left,right|left.0.cmp(right.0));
    histogram
}
fn generate(offsets:&[u64],tags:&[&str])->(Vec<u64>,serde_json::Value) {
    let begin=Instant::now();let mut phase=serde_json::Map::new();
    // Stream borrowed keys once, then order only distinct entries.
    // The resulting canonical histogram and every following operation are unchanged.
    let histogram=streaming_histogram(tags);
    std::hint::black_box(&histogram);
    let words:HashMap<_,_>=histogram.iter().enumerate().map(|(i,&(key,_))|(key,i as u64)).collect();
    let ids:Vec<u64>=tags.iter().map(|tag|words[tag]).collect();let v=words.len() as u64;let n=offsets.len()-1;
    let eligible=(3..=64).contains(&v)&&(n as u64)<=LOW&&(0..n).all(|p|{
        let slice=&ids[offsets[p] as usize..offsets[p+1] as usize];
        slice.len()<=8&&slice.iter().enumerate().all(|(i,k)|!slice[i+1..].contains(k))
    });
    phase.insert("interning_and_eligibility_seconds".into(),begin.elapsed().as_secs_f64().into());
    if !eligible {return(ordinary(offsets,&ids,v as usize),serde_json::Value::Object(phase));}
    let start=Instant::now();let (sub_offsets,sub_ids)=subsets(offsets,&ids,3);
    let (sub_counts,sub_boundaries,sub_postings)=index(&sub_offsets,&sub_ids,(v*(v-1)*(v-2)/6) as usize);
    let covered=(0..n).all(|p|(sub_offsets[p]..sub_offsets[p+1]).any(|i|sub_counts[sub_ids[i as usize]as usize]>=6));
    phase.insert("triple_index_and_coverage_seconds".into(),start.elapsed().as_secs_f64().into());
    let buckets=if v>=4 {v*(v-1)*(v-2)*(v-3)/24}else{0};
    if !covered||buckets==0||buckets>131072{return(ordinary(offsets,&ids,v as usize),serde_json::Value::Object(phase));}
    let start=Instant::now();let (quad_offsets,quad_ids)=subsets(offsets,&ids,4);
    let (_,quad_boundaries,quad_postings)=index(&quad_offsets,&quad_ids,buckets as usize);
    let masks:Vec<u64>=(0..n).map(|p|(offsets[p]..offsets[p+1]).fold(0,|mask,i|mask|1u64.wrapping_shl(ids[i as usize]as u32))).collect();
    phase.insert("quartet_index_and_masks_seconds".into(),start.elapsed().as_secs_f64().into());
    let start=Instant::now();let mut out=Vec::with_capacity(n*5);
    for query in 0..n {
        let mut top=[0u64;5];let query_mask=masks[query];
        for at in quad_offsets[query]..quad_offsets[query+1] {
            let key=quad_ids[at as usize]as usize;
            for cursor in quad_boundaries[key]..quad_boundaries[key+1] {
                let post=quad_postings[cursor as usize];let score=(query_mask&masks[post as usize]).count_ones()as u64;
                let r=rank(score,post);
                if post!=query as u64&&r>top[4]&&!top[..4].contains(&r){top=insert(top,r);}
            }
        }
        for at in sub_offsets[query]..sub_offsets[query+1] {
            if top[4]>=4u64<<58 {break;}
            let key=sub_ids[at as usize]as usize;
            for cursor in sub_boundaries[key]..sub_boundaries[key+1] {
                let post=sub_postings[cursor as usize];let r=rank(3,post);if r<=top[4]{break;}
                let inverse=LOW^post;
                if post!=query as u64&&top[..4].iter().all(|&old|(old&LOW)!=inverse){top=insert(top,r);}
            }
        }
        out.extend(top.map(|r|LOW^(r&LOW)));
    }
    phase.insert("queries_and_index_export_seconds".into(),start.elapsed().as_secs_f64().into());
    phase.insert("triple_entries".into(),sub_ids.len().into());phase.insert("quartet_entries".into(),quad_ids.len().into());phase.insert("fully_covered".into(),true.into());
    (out,serde_json::Value::Object(phase))
}
fn main()->io::Result<()> {
    let text=fs::read_to_string("../posts.json")?;let posts:Vec<Post>=serde_json::from_str(&text)?;
    // Include input flattening, allocation and generation after JSON decoding.
    let start=Instant::now();
    let mut offsets=vec![0u64];let mut tags=Vec::new();for post in &posts {tags.extend(post.tags.iter().copied());offsets.push(tags.len()as u64);}
    let (indices,phases)=generate(&offsets,&tags);
    let related:Vec<_>=posts.iter().zip(indices.chunks_exact(5)).map(|(post,top)|Related{id:post.id,tags:&post.tags,related:std::array::from_fn(|i|&posts[top[i]as usize])}).collect();
    std::hint::black_box(&related);let elapsed=start.elapsed();
    println!("Processing time (w/o IO): {:?}",elapsed);println!("{}",serde_json::json!({"diagnostic_phases":phases,"checked_release_arithmetic":true,"same_numeric_algorithm_as_mech":true,"mech_resource_accounting":false}));
    serde_json::to_writer(BufWriter::new(fs::File::create("../related_posts_rust_mech.json")?),&related)?;Ok(())
}

#[cfg(test)]
mod streaming_tests {
    use super::*;
    #[test]
    fn streaming_counts_and_order_match_full_sort_for_adversarial_keys() {
        for input in [vec![],vec![""],vec!["z","a","z","a","z"],vec!["é","e","e\0","","é","東京","a","aa","a\0"]] {
            let mut sorted=input.clone();sorted.sort_unstable();let mut expected=Vec::new();
            for key in sorted {match expected.last_mut(){Some((old,count))if *old==key=>*count+=1u64,_=>expected.push((key,1u64))}}
            assert_eq!(streaming_histogram(&input),expected);
        }
    }
}
