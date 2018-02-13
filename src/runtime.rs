// # Mech Runtime

// The Mech Runtime is the engine that drives computations in Mech. The runtime
// is comprised of "Blocks", interconnected by "Pipes" of data they query and 
// publish. Blocks can interact with the database, by Scanning for records that 
// match a pattern, or by Projecting compted records into the database.