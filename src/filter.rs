//! Filtering method that apply query and Operator that can be used to filter match between query.

use tap::tree::{Tree, TreeNodeId};
use tap::error::RustructError;

use anyhow::{anyhow, Result};
use crate::parser;

/**
 * Match query again a [nodes](tap::node::Node) list and return matching nodes. 
 */
pub struct Filter
{
}

impl Filter
{
  /// Apply `query` on all nodes in [Tree] recursively and return matching nodes [Id](TreeNodeId).
  pub fn tree(tree : &Tree, query : &str) -> Result<Vec<TreeNodeId>>
  {
    let nodes = tree.children_rec(None).unwrap();
    Filter::nodes(tree, query, &nodes)
  }

  /// Apply `query` on all nodes found in [Tree] `path` recursively and return matching nodes [Id](TreeNodeId).
  pub fn path(tree : &Tree, query : &str, path : &str) -> Result<Vec<TreeNodeId>>
  {
    let nodes = match tree.children_rec(Some(path))
    {
      Some(nodes) => nodes,
      None => return Err(anyhow!("Invalid path"))
    };
    Filter::nodes(tree, query, &nodes)
  }

  /// Apply `query` on all `nodes` and return matching Node [Id](TreeNodeId).
  #[allow(clippy::ptr_arg)]
  pub fn nodes(tree : &Tree, query : &str, nodes : &Vec<TreeNodeId>) -> Result<Vec<TreeNodeId>>
  {
    parser::OpNodesParser::new().parse(tree, nodes, query).map_err(|error| RustructError::Unknown(error.to_string()).into())
  }

}

/**
 * Implement operator (or, and, and not) for [Vec]<[TreeNodeId]>.
 */
pub struct Op
{
}

impl Op
{
  /// Apply and not operator for all element of `left` to elements of `right` and return matching nodes [Id](TreeNodeId).
  pub fn and_not(left : Vec<TreeNodeId>, right : Vec<TreeNodeId>) -> Vec<TreeNodeId>
  {
    let mut result = Vec::new();

    for id in right
    {
      if !left.contains(&id)
      {
        result.push(id);
      }
    }
    result.sort();
    result.dedup();
    result
  }

  /// Apply and operator for all element of `left` vec to elements of `right` and return matching nodes [Id](TreeNodeId). 
  pub fn and(left : Vec<TreeNodeId>, right : Vec<TreeNodeId>) -> Vec<TreeNodeId>
  {
    let mut result = Vec::new();

    for id in right
    {
      if left.contains(&id)
      {
        result.push(id); 
      }
    }
    result.sort();
    result.dedup();
    result
  }

  /// Apply or operator for all element of `left` vec to elements of `right` and return matching nodes [Id](TreeNodeId). 
  pub fn or(left : Vec<TreeNodeId>, right : Vec<TreeNodeId>) -> Vec<TreeNodeId>
  {
    let mut result = left;

    result.extend(right);
    result.sort();
    result.dedup();
    result
  }
}
