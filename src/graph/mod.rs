use std::collections::HashSet;
use std::collections::BTreeSet;
use std::iter::FromIterator;

/// A graph of nodes and directional edges represented using _adjacency lists_ that has no edge weights.
/// Adjacency lists are suitable for sparse graphs, where the number of edges is much less than 
/// the square of the number of nodes (i.e. vertices): 
/// 
///`     E << V²    `
/// 
/// To interpret the data structure, assume we have a node A, where A is the zero-based id of the node and also 
/// its position in the Vec's. 
/// 
///   - `outgoing_edges[A]` will list all the ids of nodes to which this node points via
/// an outgoing edge. 
///   - `incoming_edges[A]` will list all the ids of node pointing to this node via an incoming edge.
/// 
/// This is a bidirectional index, so every edge will appear twice; once as an incoming edge and once as an outgoing edge.
pub struct Graph {
    /// The index into the list is the `from` node Id, while the contents of the HashSet are the node Ids of related
    /// nodes we are pointing `to`, hence representing an edge from the Vec index's `from` node to the HashSet's `to` node. 
    outgoing_edges : Vec<Option<HashSet<usize>>>,

    /// The index into the list is the `to` node Id, while the contents of the HashSet are the node Ids of related
    /// nodes that the edges are pointing `from`, hence representing an edge from the HashSet's `from` node to the Vec index's `to` node. 
    incoming_edges : Vec<Option<HashSet<usize>>>
}

impl Graph {
    /// Construct a Graph whose capacity is `node_count` nodes. 
    /// The graph cannot be made larger than this. 
    pub fn new(node_count : usize) -> Self {
        Graph {
            outgoing_edges : vec!(None; node_count),
            incoming_edges : vec!(None; node_count)
        }
    }

    pub fn node_count(&self) -> usize {
        self.outgoing_edges.len()
    }

    /// Add a directional edge that starts at `from_node` and points to `to_node`.
    /// `from_node` is a dependency, whereas `to_node` is a dependent node.
    pub fn add_edge(&mut self, from_node : usize, to_node : usize) {
        self.outgoing_edges[from_node]
          .get_or_insert_with(HashSet::new)
          .insert(to_node);
        self.incoming_edges[to_node]
          .get_or_insert_with(HashSet::new)
          .insert(from_node);
    }

    /// Remove an edge that starts at `from_node` and points to `to_node`.
    /// 
    ///   - If such an edge exists and was successfully removed, return true.
    ///   - If no such edge exists, return false.
    pub fn remove_edge(&mut self, from_node : usize, to_node : usize) -> bool {
        return
          match self.outgoing_edges[from_node] {
              Some(ref mut set) => set.remove(&to_node),
              None => false
          }
          &&
          match self.incoming_edges[to_node] {
              Some(ref mut set) => set.remove(&from_node),
              None => false
          };
    }

    /// Perform a __topological sort__ of the graph, if possible.
    /// This is akin to solving a scheduling problem, where no task
    /// may be performed until all dependent tasks have been completed. 
    /// 
    ///   - Incoming edges come from nodes on which you depend. 
    ///   - Outgoing edges point to nodes that depend on you.
    /// 
    /// Returns a tuple with two `Vecs`. 
    /// 
    ///   - The first `Vec` holds ids of all the nodes that could be sorted, arranged in proper order.
    ///   - The second `Vec` holds ids of all the nodes that could NOT be sorted.
    /// 
    /// __Cyclic or Acyclic__? If the second `Vec` returned is empty, it means the graph 
    /// is a __DAG__ (`directed acyclic graph`) and a complete topological sort was possible. 
    /// _Otherwise, the graph had cycles_. 
    /// 
    /// __Non-unique solution__. This topological ordering is not unique; other valid orderings may be possible. 
    /// 
    /// __Disconnected graphs__. The graph does not need to be fully connected to be sorted; 
    /// it can consist of separate graphs that have no edges joining them. 
    /// 
    /// __Already sorted lists__. If the graph is defined such that sorting the nodes by ascending node ID produces a
    /// valid topological sorting order, then that is the order that will be returned,
    /// and that solution will be obtained efficiently. 
    /// e.g. an already sorted list will remain in the same order. 
    /// 
    /// __Destructive sort__. This method destroys the graph in the process of sorting. 
    /// 
    /// __Algorithm__. 
    /// 
    ///   - Loop, until no forward progress.
    ///       - Reset forward progress to false.
    ///       - Loop over all nodes in ascending node id order.
    ///         - If node has no dependencies: 
    ///           - add node to the solution as sortable, 
    ///           - remove all node's outgoing edges,
    ///           - set forward progress to true.
    ///   - Loop over all nodes
    ///     - If node not in sortable, add to unsortable 
    ///   - Return sortable and unsortable Vecs.  
    pub fn sort(mut self) -> (Vec<usize>, Vec<usize>) {
        let nodes = self.node_count();
        // BTreeSet necessary, because to guarantee that an already sorted graph 
        // retains the same ordering, we need to iterate over the ids in sorted order.
        let mut unsorted : BTreeSet<usize> = BTreeSet::from_iter(0..nodes);
        let mut sortable = Vec::with_capacity(nodes);
        let mut forward_progress = true;
        
        while forward_progress {
            forward_progress = false;
            let mut dependency_id_to_remove = None;
            
            for dependency_id in unsorted.iter() {
                if self.incoming_edges[*dependency_id].is_none() {
                    // The clone is needed, because remove_edge modifies outgoing_edges. 
                    if let Some(dependencies) = &self.outgoing_edges[*dependency_id] {
                        for dependent_id in dependencies.clone() {
                            self.remove_edge(*dependency_id, dependent_id);
                        }  
                    }
                    dependency_id_to_remove = Some(*dependency_id);
                    // Why not continue in this loop? If we did, we could not guarantee 
                    // that acceptably ordered nodes would remain in the same order. 
                    // Example: 
                    //   A has no dependencies
                    //   B depends on A
                    //   C has no dependencies
                    // If we do not break here, our ordering is A, C, B.
                    // If we do break here, our ordering is A, B, C. 
                    break;
                }
            }
            
            // Perform remove outside of loop to prevent modifying a collection while it is being iterated.
            if let Some(sortable_id) = dependency_id_to_remove {
                sortable.push(sortable_id);
                unsorted.remove(&sortable_id);
                forward_progress = true;
            }
        }
    
        let unsortable = unsorted.iter().map(|i| *i).collect();
        (sortable, unsortable)
    }
}

mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[allow(unused_imports)]
    use spectral::prelude::*;


    #[test]
    /// Perform topological sort of the nodes in a DAG.
    pub fn topological_sort_no_cycles() {
        // Expected order: 0, 1, 2, 3, 4, 5, 6
        let mut graph = Graph::new(7);
        graph.add_edge(2, 6);
        graph.add_edge(2, 5);
        graph.add_edge(1, 3);
        graph.add_edge(3, 4);
        let (ordered, unordered) = graph.sort();
        asserting("Should be no unordered nodes").that(&unordered.len()).is_equal_to(0);
        let ordered_comparison = ordered.iter().eq(vec!(0,1,2,3,4,5,6).iter());
        asserting("Ordered node ids should be ascending").that(&ordered_comparison).is_equal_to(true);
    }

}
