//! Generate a timeline from a vector of [nodes](Node).

use tap::node::Node;
use tap::value::ValueTypeId;
use tap::attribute::Attribute;
use tap::tree::{Tree, TreeNodeId};

use serde::Serialize;
use chrono::{DateTime, Utc};
use rayon::prelude::*;
use anyhow::{anyhow, Result};

/// Contain `time` a [DateTime] [value](tap::value::Value) of the [Attribute] named `attribute_name` found in node `id`.
#[derive(Serialize)]
pub struct TimeInfo 
{
  pub time : DateTime<Utc>,
  pub attribute_name : String,
  pub id : TreeNodeId,
}

/**
 *  Search for all [DateTime] [Attribute] on each [Node] of a Vector of [Node] 
 *  then return a sorted Vector of [TimeInfo] for each [DateTime] [Attribute] found on each [Node]
 *  creating a timeline (a node would generate multiple [TimeInfo] one for each of it's [DateTime] [Attribute] .)
 */
pub struct Timeline
{
}

impl Timeline
{
  /// Return a timeline as a [Vec]<[TimeInfo]> containing all [DateTime] [Attribute] which time is included between `min_time` and `max_time` for all [nodes](Node) in the [Tree].
  pub fn tree(tree : &Tree, min_time : &DateTime<Utc>, max_time : &DateTime<Utc>) -> Vec<TimeInfo>
 {
    let nodes = tree.children_rec(None).unwrap();
    Timeline::nodes(&tree, &nodes, min_time, max_time)
  }

  /// Return a timeline as a [Vec]<[TimeInfo]> containing all [DateTime] [Attribute] which time is included between min_time and max_time for all [nodes](Node) that can be found recursively from `path`.
  pub fn path(tree : &Tree, path : &str,  min_time : &DateTime<Utc>, max_time : &DateTime<Utc>) -> Result<Vec<TimeInfo>>
  {
    let nodes = match tree.children_rec(Some(path))
    {
      Some(nodes) => nodes,
      None => return Err(anyhow!("Invalid path"))
    };
    Ok(Timeline::nodes(&tree, &nodes, min_time, max_time))
  }

  /// Return a timeline as a [Vec]<[TimeInfo]> containing all [DateTime] [Attribute] which time is included between min_time and max_time for all `nodes`[TreeNodeId].
  pub fn nodes(tree : &Tree, nodes : &Vec<TreeNodeId>, min_time : &DateTime<Utc>, max_time : &DateTime<Utc>) -> Vec<TimeInfo>
  {
    let mut times : Vec<TimeInfo> =  nodes.par_iter().filter_map(|node_id|
    {
      if let Some(node) = tree.get_node_from_id(*node_id)
      {
        return Some(Timeline::match_time(&node, &node_id, &min_time, &max_time))
      }
      else
      {
        None
      }
    }).flatten().collect();

    times.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    times
  }

  fn match_time(node : &Node, node_id : &TreeNodeId, min_time : &DateTime<Utc>, max_time : &DateTime<Utc> ) -> Vec<TimeInfo>
  {      
    let mut times = Vec::new();
    for attribute in node.value().attributes().iter()
    {
      Timeline::match_time_rec("".into(), &node_id, &attribute, &mut times, &min_time, &max_time);
    }
    times
  }

  fn match_time_rec(dotted_attrib: String, node_id : &TreeNodeId, attribute : &Attribute, mut times : &mut Vec<TimeInfo>, min_time : &DateTime<Utc>, max_time : &DateTime<Utc>)
  {
    if attribute.type_id() == ValueTypeId::Attributes
    {
      for current_attribute in attribute.value().as_attributes().attributes().iter()
      {
        let dotted_attrib = match dotted_attrib.len() 
        {
          0 => attribute.name().to_string(),
          _ => dotted_attrib.to_string() + "." + attribute.name(),
        };
        Timeline::match_time_rec(dotted_attrib, &node_id, &current_attribute, &mut times, &min_time, &max_time)
      }
    }
    else if attribute.type_id() == ValueTypeId::ReflectStruct 
    {
      let attributes : Vec<Attribute> = attribute.value().as_reflect_struct().attributes();
      for current_attribute in attributes.iter() 
      {
        let dotted_attrib = match dotted_attrib.len() 
        {
          0 => attribute.name().to_string(),
          _ => dotted_attrib.to_string() + "." + attribute.name(),
        };
        Timeline::match_time_rec(dotted_attrib, &node_id, &current_attribute, &mut times, &min_time, &max_time)
      }
    }
    else if attribute.type_id() == ValueTypeId::DateTime
    {
      let attribute_time = attribute.value().as_date_time();
      if (attribute_time  >= *min_time && attribute_time <= *max_time)
      {
        match dotted_attrib.len() 
        {
          0 => times.push(TimeInfo{time : attribute_time, id : *node_id, attribute_name : attribute.name().to_string()}),
          _ => times.push(TimeInfo{time : attribute_time, id : *node_id, attribute_name : dotted_attrib + "." + attribute.name()}),
        }
      }
    }
  }
}
