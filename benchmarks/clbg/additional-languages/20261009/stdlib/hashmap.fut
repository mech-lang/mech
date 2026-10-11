-- Growing, open-addressed u64 -> i64 hash table.  Zero is a valid key.
type~ table = ([](u64,i64,bool),i64)
def empty (capacity:i64): *table =
  (replicate capacity (0u64,0i64,false),0i64)
def hash (x:u64):u64 =
  let x = (x ^ (x >> 30))*0xbf58476d1ce4e5b9u64
  let x = (x ^ (x >> 27))*0x94d049bb133111ebu64
  in x ^ (x >> 31)
def slot (slots:[](u64,i64,bool)) (key:u64):i64 =
  let mask = length slots-1
  in loop i = i64.u64 (hash key & u64.i64 mask)
     while slots[i].2 && slots[i].0 != key do (i+1)&mask
def get (t:table) (key:u64):i64 =
  let (slots,_count) = t
  let i = slot slots key
  in if slots[i].2 then slots[i].1 else 0
def put_raw (t:*table) (key:u64) (value:i64):*table =
  let (slots,count) = t
  let i = slot slots key
  let count = count + (if slots[i].2 then 0 else 1)
  let slots[i] = (key,value,true)
  in (slots,count)
def grow (t:*table):*table =
  let (slots,_count) = t
  in loop fresh = empty (length slots*2) for i < length slots do
       if slots[i].2 then put_raw fresh slots[i].0 slots[i].1 else fresh
def put (t:*table) (key:u64) (value:i64):*table =
  let t = if (t.1+1)*10 > length t.0*7 then #[noinline] grow t else t
  in put_raw t key value
