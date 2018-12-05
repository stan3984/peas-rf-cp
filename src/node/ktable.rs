
use std::net::SocketAddr;
use common::id::Id;

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Debug, Hash)]
// TODO: implement getters
// TODO: implement new and stuff
pub struct Entry {
    sock: SocketAddr,
    id: Id,
}

pub struct Ktable {
    table: Vec<Vec<Entry>>,
    k: u32,
    id: Id,
}

impl Entry {
    pub fn new(sock: SocketAddr, id: Id) -> Self {
        Entry {sock: sock, id: id}
    }

}

impl Entry {
    pub fn new(sock: SocketAddr, id: Id) -> Self {
        unimplemented!()
    }
    pub fn get_id(&self) -> Id {
        self.id
    }
    pub fn get_addr(&self) -> SocketAddr {
        self.sock
    }
}

impl Ktable {
    pub fn new(k: u32, me: Id) -> Self {
        // use Id length ??
        Ktable {table: vec![Vec::new(); 64], k: k, id: me}
    }
    pub fn offer(&mut self, offer: Entry) {
        debug!("ktable got {:?} offered", offer);
        if offer.id == self.id{
            return;
        }
        let (v1_index, v2_index, found) = self.index_from_id(offer.id);
        if !found {
            if self.table[v1_index].len() < self.k as usize {
                self.table[v1_index].insert(v2_index, offer);
            }
        }
    }
    pub fn offer_replace(&mut self, offer: Entry) {
        debug!("ktable deleted {:?}", entry);
        if offer.id == self.id{
            return;
        }
        let (v1_index, v2_index, found) = self.index_from_id(offer.id);
        if !found {
            self.table[v1_index].insert(v2_index, offer);
            if self.table[v1_index].len() as u32 > self.k{
                self.table[v1_index].pop();
            }
        }
    }
    pub fn delete_id(&mut self, id: Id){
        if id == self.id{
            return;
        }
        let (v1_index, v2_index, found) = self.index_from_id(id);
        if found {
            self.table[v1_index].remove(v2_index);
        }
    }
    pub fn delete_entry(&mut self, entry: Entry) {
        if entry.id == self.id{
            return;
        }
        let (v1_index, v2_index, found) = self.index_from_id(entry.id);
        if found {
            if entry == self.table[v1_index][v2_index]{
                self.table[v1_index].remove(v2_index);
            }
        }
    }
    pub fn clear(&mut self) {
        self.table.clear();
    }
    pub fn get(&self, n: u32) -> Vec<Entry> {
        let mut result:Vec<Entry> = Vec::new();
        let mut counter = 0;
        'get_loop:
        for v1_entry in self.table.iter().rev(){
            for v2_entry in v1_entry {
                result.push(*v2_entry);
                counter += 1;
                if counter == n {
                    break 'get_loop;
                }
            }
        }
        return result;
    }
    pub fn closest_to(&self, n: u32, other: Id) -> Vec<Entry> {
        let mut result:Vec<Entry> = Vec::new();
        //Iterate through all entries
        for v1_entry in self.table.iter(){
            for v2_entry in v1_entry{
                //Sort and insert entries
                let mut index:usize = 0;
                for result_entry in result.iter() {
                    if v2_entry.id.distance(&other) > result_entry.id.distance(&other) {
                        index += 1;
                    } else {
                        break;
                    }
                }
                if index as u32 <= n {
                    result.insert(index, *v2_entry);
                }
                if result.len() as u32 > n {
                    result.pop();
                }
            }
        }
        return result;
    }
    fn index_from_id(&self, id: Id) -> (usize, usize, bool){
        let mut found = false;
        let v1_index:usize = self.id.common_bits(&id) as usize;
        let dist_id = self.id.distance(&id);
        let mut v2_index:usize = 0;
        while v2_index < self.table[v1_index].len() {
            let dist_entry = self.id.distance(&self.table[v1_index][v2_index].id);
            if dist_id > dist_entry{
                v2_index += 1;
            }
            else if dist_id == dist_entry{
                found = true;
                break;
            }
            else{
                break;
            }
        }
        (v1_index, v2_index, found)
    }
}
