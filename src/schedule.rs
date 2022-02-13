
use hashbrown::HashMap;
use crate::core::BlockRef;
use crate::*;

#[derive(Clone)]
pub struct Schedule {
  pub input_to_block: HashMap<TableId,(TableIndex,TableIndex,Vec<BlockRef>)>,
  pub output_to_block: HashMap<TableId,(TableIndex,TableIndex,Vec<BlockRef>)>,
  unscheduled_blocks: Vec<BlockRef>,
  pub schedules: HashMap<(TableId,TableIndex,TableIndex),BlockGraph>, // Block Graph is list of blocks that will trigger in order when the given register is set
}

impl Schedule {

  pub fn new() -> Schedule {
    Schedule {
      input_to_block: HashMap::new(),
      output_to_block: HashMap::new(),
      schedules: HashMap::new(),
      unscheduled_blocks: Vec::new(),
    }
  }

  pub fn add_block(&mut self, block_ref: BlockRef) -> Result<(),MechError> {

    self.unscheduled_blocks.push(block_ref);

    Ok(())
  }

  pub fn schedule_blocks(&mut self) -> Result<(),MechError> {
    for block_ref in &self.unscheduled_blocks {
      let block_brrw = block_ref.borrow();
      // Map input tables to blocks
      for (table_id,row,col) in &block_brrw.input {
        let (_,_,ref mut dependent_blocks) = self.input_to_block.entry(*table_id).or_insert((*row,*col,vec![]));
        dependent_blocks.push(block_ref.clone());
        self.schedules.insert((*table_id,*row,*col),BlockGraph::new(block_ref.clone()));
      }

      // Map output tables to blocks
      for (table_id,row,col) in &block_brrw.output {
        let (_,_,ref mut dependent_blocks) = self.output_to_block.entry(*table_id).or_insert((*row,*col,vec![]));
        dependent_blocks.push(block_ref.clone());
      }

    }
    Ok(())
  }

  pub fn run_schedule(&mut self, register: &(TableId,TableIndex,TableIndex)) -> Result<(),MechError> {
    match self.schedules.get_mut(register) {
      Some(ref mut block_graph) => {
        block_graph.solve();
        return Ok(())
      }
      None => {
        return Err(MechError::GenericError(8519))
      }
    }
  }
}

impl fmt::Debug for Schedule {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let mut box_drawing = BoxPrinter::new();
    box_drawing.add_header("input");
    box_drawing.add_line(format!("{:?}", &self.input_to_block.iter().map(|(table_id,(row,col,dependent_blocks))| {
      ((*table_id,*row,*col),dependent_blocks.iter().map(|b| humanize(&b.borrow().id)).collect::<Vec<String>>())
    }).collect::<Vec<((TableId,TableIndex,TableIndex),Vec<String>)>>()));
    box_drawing.add_header("output");
    box_drawing.add_line(format!("{:?}", &self.output_to_block.iter().map(|(table_id,(row,col,dependent_blocks))| {
      ((*table_id,*row,*col),dependent_blocks.iter().map(|b| humanize(&b.borrow().id)).collect::<Vec<String>>())
    }).collect::<Vec<((TableId,TableIndex,TableIndex),Vec<String>)>>()));
    box_drawing.add_header("schedules");
    box_drawing.add_line(format!("{:#?}", &self.schedules));
    box_drawing.add_header("unscheduled blocks");
    box_drawing.add_line(format!("{:#?}", &self.unscheduled_blocks.iter().map(|b| humanize(&b.borrow().id)).collect::<Vec<String>>()));
    write!(f,"{:?}",box_drawing)?;
    Ok(())
  }
}


#[derive(Clone)]
struct Node {
  block: BlockRef,
  parents: Vec<Node>,
  children: Vec<Node>,
}

impl Node {

  pub fn new(block: BlockRef) -> Node {
    Node {
      block: block,
      parents: Vec::new(),
      children: Vec::new(),
    }
  }

  pub fn solve(&mut self) -> Result<(),MechError> {
    self.block.borrow_mut().solve()?;
    for ref mut child in &mut self.children {
      child.solve()?;
    }
    Ok(())
  }

}

impl fmt::Debug for Node {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let mut box_drawing = BoxPrinter::new();
    box_drawing.add_line(format!("{:?}", humanize(&self.block.borrow().id)));
    write!(f,"{:?}",box_drawing)?;
    Ok(())
  }
}

#[derive(Clone,Debug)]
pub struct BlockGraph {
  root: Node,
}


impl BlockGraph {

  pub fn new(block: BlockRef) -> BlockGraph {
    let node = Node::new(block);
    BlockGraph {
      root: node,
    }
  }

  pub fn insert_block(&mut self, block: BlockRef) -> Result<(),MechError> {
    Ok(())
  }

  pub fn solve(&mut self) -> Result<(),MechError> {
    self.root.solve()
  }


}