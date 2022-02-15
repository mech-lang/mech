
use hashbrown::HashMap;
use crate::core::BlockRef;
use crate::*;

#[derive(Clone)]
pub struct Schedule {
  pub input_to_block: HashMap<(TableId,TableIndex,TableIndex),Vec<BlockGraph>>,
  pub output_to_block: HashMap<(TableId,TableIndex,TableIndex),Vec<BlockGraph>>,
  pub schedules: HashMap<(TableId,TableIndex,TableIndex),BlockGraph>, // Block Graph is list of blocks that will trigger in order when the given register is set
  unscheduled_blocks: Vec<BlockRef>,
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
      let mut graph = BlockGraph::new(block_ref.clone());
      let block_brrw = block_ref.borrow();

      // Map input tables to blocks
      for (input_table_id,row,col) in &block_brrw.input {
        let ref mut dependent_blocks = self.input_to_block.entry((*input_table_id,*row,*col)).or_insert(vec![]);
        dependent_blocks.push(graph.clone());
        self.schedules.insert((*input_table_id,*row,*col),graph.clone());
        for ((output_table_id,row,col),ref mut producing_blocks) in self.output_to_block.iter_mut() {
          if output_table_id == input_table_id {
            for ref mut pblock in producing_blocks.iter_mut() {
              println!("{:?}>=={:?}==>>{:?}", pblock, input_table_id, humanize(&block_brrw.id));
              pblock.insert_block(&mut graph);
            }
          }
        }
      }

      // Map output tables to blocks
      for (table_id,row,col) in &block_brrw.output {
        let ref mut dependent_blocks = self.output_to_block.entry((*table_id,*row,*col)).or_insert(vec![]);
        dependent_blocks.push(graph.clone());
      }

    }
    self.unscheduled_blocks.clear();
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
    box_drawing.add_line(format!("{:#?}", &self.input_to_block));
    box_drawing.add_header("output");
    box_drawing.add_line(format!("{:#?}", &self.output_to_block));
    box_drawing.add_header("schedules");
    box_drawing.add_line(format!("{:#?}", &self.schedules));
    box_drawing.add_header("unscheduled blocks");
    box_drawing.add_line(format!("{:#?}", &self.unscheduled_blocks.iter().map(|b| humanize(&b.borrow().id)).collect::<Vec<String>>()));
    write!(f,"{:?}",box_drawing)?;
    Ok(())
  }
}


#[derive(Clone)]
pub struct Node {
  block: BlockRef,
  parents: Vec<Rc<RefCell<Node>>>,
  children: Vec<Rc<RefCell<Node>>>,
}

impl Node {

  pub fn new(block: BlockRef) -> Node {
    Node {
      block: block,
      parents: Vec::new(),
      children: Vec::new(),
    }
  }

  pub fn add_child(&mut self, child: Rc<RefCell<Node>>) {
    self.children.push(child);
  }

  pub fn add_parent(&mut self, parent: Rc<RefCell<Node>>) {
    self.parents.push(parent);
  }

  pub fn solve(&mut self) -> Result<(),MechError> {
    self.block.borrow_mut().solve()?;
    for ref mut child in &mut self.children {
      child.borrow_mut().solve()?;
    }
    Ok(())
  }

}

impl fmt::Debug for Node {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f,"{:?}",humanize(&self.block.borrow().id))?;
    write!(f,"{:?}",&self.children)?;
    Ok(())
  }
}

#[derive(Clone)]
pub struct BlockGraph {
  pub root: Rc<RefCell<Node>>,
}

impl BlockGraph {

  pub fn new(block: BlockRef) -> BlockGraph {
    let node = Rc::new(RefCell::new(Node::new(block)));
    BlockGraph {
      root: node,
    }
  }

  pub fn insert_block(&mut self, block: &mut BlockGraph) -> Result<(),MechError> {
    {
      let mut root_block = self.root.borrow_mut();
      let rc = block.root.clone();
      root_block.add_child(rc);
    }
    {
      let mut child_block = block.root.borrow_mut();
      child_block.add_parent(self.root.clone());
    }
    Ok(())
  }

  pub fn solve(&mut self) -> Result<(),MechError> {
    self.root.borrow_mut().solve()
  }


}

impl fmt::Debug for BlockGraph {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f,"[{:?}]",self.root.borrow())?;
    Ok(())
  }
}