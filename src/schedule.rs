
use hashbrown::HashMap;
use crate::core::BlockRef;
use crate::*;

#[derive(Clone)]
pub struct Schedule {
  pub trigger_to_blocks: HashMap<(TableId,TableIndex,TableIndex),Vec<BlockGraph>>,
  pub input_to_blocks: HashMap<(TableId,TableIndex,TableIndex),Vec<BlockGraph>>,
  pub output_to_blocks: HashMap<(TableId,TableIndex,TableIndex),Vec<BlockGraph>>,
  pub schedules: HashMap<(TableId,TableIndex,TableIndex),Vec<BlockGraph>>, // Block Graph is list of blocks that will trigger in order when the given register is set
  unscheduled_blocks: Vec<BlockRef>,
}

impl Schedule {

  pub fn new() -> Schedule {
    Schedule {
      trigger_to_blocks: HashMap::new(),
      input_to_blocks: HashMap::new(),
      output_to_blocks: HashMap::new(),
      schedules: HashMap::new(),
      unscheduled_blocks: Vec::new(),
    }
  }
 
  pub fn add_block(&mut self, block_ref: BlockRef) -> Result<(),MechError> {

    self.unscheduled_blocks.push(block_ref);

    Ok(())
  }

  pub fn schedule_blocks(&mut self) -> Result<(),MechError> {
    let ready_blocks: Vec<BlockRef> = self.unscheduled_blocks.drain_filter(|b| b.borrow().state == BlockState::Ready).collect();

    for block_ref in &ready_blocks {
      let mut graph = BlockGraph::new(block_ref.clone());
      let block_brrw = block_ref.borrow();

      // Map trigger registers to blocks
      for (trigger_table_id,row,col) in &block_brrw.triggers {
        let ref mut dependent_blocks = self.trigger_to_blocks.entry((*trigger_table_id,*row,*col)).or_insert(vec![]);
        dependent_blocks.push(graph.clone());
        let ref mut dependent_blocks = self.schedules.entry((*trigger_table_id,*row,*col)).or_insert(vec![]);
        dependent_blocks.push(graph.clone());

        for ((output_table_id,row,col),ref mut producing_blocks) in self.output_to_blocks.iter_mut() {
          if output_table_id == trigger_table_id {
            for ref mut pblock in producing_blocks.iter_mut() {
              pblock.load_block(&mut graph);
            }
          }
        }
      }

      // Map input registers to blocks
      for (input_table_id,row,col) in &block_brrw.input {
        let ref mut dependent_blocks = self.input_to_blocks.entry((*input_table_id,*row,*col)).or_insert(vec![]);
        dependent_blocks.push(graph.clone());
      }

      // Map output registers to blocks
      for (output_table_id,row,col) in &block_brrw.output {
        let ref mut dependent_blocks = self.output_to_blocks.entry((*output_table_id,*row,*col)).or_insert(vec![]);
        dependent_blocks.push(graph.clone());
      }

    }
    Ok(())
  }

  pub fn run_schedule(&mut self, register: &(TableId,TableIndex,TableIndex)) -> Result<(),MechError> {
    match self.schedules.get_mut(register) {
      Some(ref mut block_graphs) => {
        for ref mut block_graph in block_graphs.iter_mut() {
          block_graph.solve();
        }
        Ok(())
      }
      None => {
        Err(MechError{id: 5368, kind: MechErrorKind::GenericError(format!("No schedule assocaited with {:?}", register))})
      }
    }
  }
}

impl fmt::Debug for Schedule {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let mut box_drawing = BoxPrinter::new();
    box_drawing.add_header("triggers");
    box_drawing.add_line(format!("{:#?}", &self.trigger_to_blocks));
    box_drawing.add_header("input");
    box_drawing.add_line(format!("{:#?}", &self.input_to_blocks));
    box_drawing.add_header("output");
    box_drawing.add_line(format!("{:#?}", &self.output_to_blocks));
    box_drawing.add_header("schedules");
    box_drawing.add_line(format!("{:#?}", &self.schedules));
    if self.unscheduled_blocks.len() > 0 {
      box_drawing.add_header("unscheduled blocks");
      box_drawing.add_line(format!("{:#?}", &self.unscheduled_blocks.iter().map(|b| humanize(&b.borrow().id)).collect::<Vec<String>>()));
    }
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
    write!(f,"[{}]",humanize(&self.block.borrow().id))?;
    for child in &self.children {
      write!(f,"--->{:?}\n",&child.borrow())?;
    }
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

  pub fn load_block(&mut self, block: &mut BlockGraph) -> Result<(),MechError> {
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
    write!(f,"{:?}",self.root.borrow())?;
    Ok(())
  }
}