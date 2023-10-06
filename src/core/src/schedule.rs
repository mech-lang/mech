/*

This code defines a scheduling system for blocks in the Mech programming language. The Schedule struct maps triggers, inputs, and outputs to blocks and maintains schedules for output-producing blocks. The Node and BlockGraph structs represent the nodes and edges of the directed acyclic graph formed by the blocks, respectively. The scheduling algorithm determines when blocks can be scheduled for execution based on their input dependencies, and their outputs are aggregated and propagated to dependent blocks. Overall, this code is a crucial component of the Mech language, allowing for the efficient and reliable execution of complex programs.

*/

use hashbrown::{HashMap, HashSet};
use crate::*;

/*
The Schedule struct is used to manage the scheduling of Blocks in Mech. It contains several maps that relate registers (i.e. specific cells in a Table) to their corresponding Blocks. The Schedule struct also has methods for adding Blocks to the schedule, scheduling those Blocks, and running the scheduled Blocks. The Schedule struct manages the flow of data between Blocks and ensures that Blocks are only executed when their input data is available. Finally, the Schedule struct contains a BlockGraph struct, which is used to represent the graph of Blocks and their dependencies.
*/

#[derive(Clone)]
pub struct Schedule {
  pub trigger_to_blocks: HashMap<(TableId,RegisterIndex,RegisterIndex),Vec<BlockGraph>>,
  pub input_to_blocks: HashMap<(TableId,RegisterIndex,RegisterIndex),Vec<BlockGraph>>,
  pub output_to_blocks: HashMap<(TableId,RegisterIndex,RegisterIndex),Vec<BlockGraph>>,
  pub trigger_to_output: HashMap<(TableId,RegisterIndex,RegisterIndex),HashSet<(TableId,RegisterIndex,RegisterIndex)>>,
  pub schedules: HashMap<(TableId,RegisterIndex,RegisterIndex),Vec<BlockGraph>>, // Block Graph is list of blocks that will trigger in order when the given register is set
  unscheduled_blocks: Vec<BlockRef>,
}

impl Schedule {

  pub fn new() -> Schedule {
    Schedule {
      trigger_to_blocks: HashMap::new(),
      input_to_blocks: HashMap::new(),
      output_to_blocks: HashMap::new(),
      trigger_to_output: HashMap::new(),
      schedules: HashMap::new(),
      unscheduled_blocks: Vec::new(),
    }
  }
 
  pub fn add_block(&mut self, block_ref: BlockRef) -> Result<(),MechError> {

    self.unscheduled_blocks.push(block_ref);

    Ok(())
  }


  pub fn schedule_blocks(&mut self) -> Result<(),MechError> {
    if  self.unscheduled_blocks.len() == 0 {
      return Ok(())
    }
    let ready_blocks: Vec<BlockRef> = self.unscheduled_blocks.extract_if(|b| b.borrow().state == BlockState::Ready).collect();

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
              pblock.add_child(&mut graph);
            }
          }
        }
      }

      // Map input registers to blocks
      for (input_table_id,row,col) in &block_brrw.input {
        let ref mut consuming_blocks = self.input_to_blocks.entry((*input_table_id,*row,*col)).or_insert(vec![]);
        consuming_blocks.push(graph.clone());
      }

      // Map output registers to blocks
      for (output_table_id,row,col) in &block_brrw.output {
        let ref mut producing_blocks = self.output_to_blocks.entry((*output_table_id,*row,*col)).or_insert(vec![]);
        producing_blocks.push(graph.clone());
        // Map block outputs to triggers
        if let Some(consuming_blocks) = self.trigger_to_blocks.get(&(*output_table_id,*row,*col)) {
          for block in consuming_blocks {
            graph.add_child(&block);
          }
        }
      }
    }
    // This collects all of the output that would be changed given a trigger
    // TODO I'd like to do this incrementally instead of redoing it
    // every time blocks are scheduled. But I'm short on time now and 
    // this is all I can think of to do without changing too much.
    for (register,block_graphs) in self.schedules.iter() {
      let (table_id,row_ix,col_ix) = register;
      let mut aggregate_output = HashSet::new();
      for graph in block_graphs {
        let mut node = &graph.root;
        let node_brrw = node.borrow();
        let mut output = node_brrw.aggregate_output();
        aggregate_output = aggregate_output.union(&mut output).cloned().collect();
      }
      self.trigger_to_output.insert(register.clone(),aggregate_output);
    }
    Ok(())
  }

  pub fn run_schedule(&mut self, register: &(TableId,RegisterIndex,RegisterIndex)) -> Result<(),MechError> {
    match self.schedules.get_mut(register) {
      Some(ref mut block_graphs) => {
        for ref mut block_graph in block_graphs.iter_mut() {
          block_graph.solve();
        }
        Ok(())
      }
      None => {
        Err(MechError{msg: "".to_string(), id: 5368, kind: MechErrorKind::GenericError(format!("No schedule assocaited with {:?}", register))})
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
    box_drawing.add_header("output schedule");
    box_drawing.add_line(format!("{:#?}", &self.trigger_to_output));
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


/*
The Node struct is used in the BlockGraph to represent a single block in the graph. It contains a BlockRef which refers to the block represented by the node, a vector of parents which represent the nodes that depend on this node, and a vector of children which represent the nodes that this node depends on. The Node also contains several methods to manipulate and retrieve the inputs, outputs, and triggers of the block, and to solve the block and its children recursively.
*/

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

  pub fn recompile(&self) -> Result<(),MechError> {
    {
      self.block.borrow_mut().recompile()?;
    }
    for child in &self.children {
      let mut child_brrw = child.borrow_mut();
      child_brrw.recompile()?;
    }
    Ok(())
  }

  pub fn triggers(&self) -> HashSet<(TableId,RegisterIndex,RegisterIndex)> {
    self.block.borrow().triggers.clone()
  }

  pub fn input(&self) -> HashSet<(TableId,RegisterIndex,RegisterIndex)> {
    self.block.borrow().input.clone()
  }

  pub fn output(&self) -> HashSet<(TableId,RegisterIndex,RegisterIndex)> {
    self.block.borrow().output.clone()
  }

  pub fn aggregate_output(&self) -> HashSet<(TableId,RegisterIndex,RegisterIndex)> {
    let mut aggregate_output = self.output();
    let mut child_output = self.output_recurse();
    aggregate_output = aggregate_output.union(&mut child_output).cloned().collect();
    aggregate_output
  }

  pub fn output_recurse(&self) -> HashSet<(TableId,RegisterIndex,RegisterIndex)> {
    let mut aggregate_output = HashSet::new();
    for child in &self.children {
      let child_brrw = child.borrow();
      let mut output = child_brrw.output();
      let mut child_output = child_brrw.output_recurse();
      aggregate_output = aggregate_output.union(&mut output).cloned().collect();
      aggregate_output = aggregate_output.union(&mut child_output).cloned().collect();
    }
    aggregate_output
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
      write!(f,"->{:?}\n",&child.borrow())?;
    }
    Ok(())
  }
}

/*
The BlockGraph structure represents a directed acyclic graph (DAG) of Node structures. The Node structures themselves contain BlockRef references, which are references to individual Block structures. Each Node also contains vectors of its parent and child Nodes within the BlockGraph. The BlockGraph provides methods for recompiling its associated Block structures, as well as for solving the BlockGraph in the proper order, with each Block solving its inputs before solving its outputs.
*/

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

  pub fn id(&self) -> u64 {
    self.root.borrow().block.borrow().id
  }

  pub fn recompile_blocks(&self) -> Result<(),MechError> {
    let root_brrw = self.root.borrow();
    root_brrw.recompile()?;
    Ok(())
  }

  pub fn triggers(&self) -> HashSet<(TableId,RegisterIndex,RegisterIndex)> {
    self.root.borrow().triggers()
  }

  pub fn input(&self) -> HashSet<(TableId,RegisterIndex,RegisterIndex)> {
    self.root.borrow().input()
  }

  pub fn output(&self) -> HashSet<(TableId,RegisterIndex,RegisterIndex)> {
    self.root.borrow().output()
  }

  pub fn add_child(&mut self, block: &BlockGraph) -> Result<(),MechError> {
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