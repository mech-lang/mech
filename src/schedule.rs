struct Schedule {
    pub input_to_block: HashMap<TableId,(TableIndex,TableIndex,Vec<BlockRef>)>,
    pub output_to_block: HashMap<TableId,(TableIndex,TableIndex,Vec<BlockRef>)>,
}